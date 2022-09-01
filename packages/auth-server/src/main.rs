use askama::Template;
use axum::{
    extract::Form,
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
    Extension, Router,
};
use serde::Deserialize;
use std::{collections::HashMap, iter::Map, net::SocketAddr, sync::Arc};
use tokio::sync::{Mutex, RwLock};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Deserialize)]
struct Login {
    email: String,
    password: String,
}

#[derive(Eq, Hash, PartialEq, Clone, Debug)]
struct UserId(u32);

#[derive(Eq, Hash, PartialEq, Clone, Debug)]
struct UserEmail(String);

#[derive(Eq, Hash, PartialEq, Clone, Debug)]
struct User {
    id: UserId,
    email: UserEmail,
    password: String,
}

#[derive(Clone, Debug)]
struct UserEmailMap(HashMap<UserEmail, User>);

/// 認可コード
#[derive(Eq, Hash, PartialEq, Clone, Debug)]
struct AuthorizationCode(String);

#[derive(Clone, Debug)]
struct AbleableAuthorizationCodeMap(HashMap<AuthorizationCode, User>);

#[derive(Eq, Hash, PartialEq, Clone, Debug)]
struct AccessToken(String);

#[derive(Clone, Debug)]
struct AbleableAccessTokenMap(HashMap<AccessToken, User>);

#[derive(Clone, Debug)]
struct Store {
    userEmailMap: Arc<Mutex<UserEmailMap>>,
    ableableAuthorizationCodeMap: Arc<Mutex<AbleableAuthorizationCodeMap>>,
    ableableAccessTokenMap: Arc<Mutex<AbleableAccessTokenMap>>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "auth-server=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();
    let mut users = UserEmailMap(HashMap::new());
    let user_email_1 = UserEmail("sadness_ojisan@example.com".to_string());
    let user_1 = User {
        id: UserId(1),
        email: user_email_1.clone(),
        password: "sadness_ojisan".to_string(),
    };
    users.0.insert(user_email_1, user_1);
    let store: Store = Store {
        userEmailMap: Arc::new(Mutex::new(users)),
        ableableAccessTokenMap: Arc::new(Mutex::new(AbleableAccessTokenMap(HashMap::new()))),
        ableableAuthorizationCodeMap: Arc::new(Mutex::new(AbleableAuthorizationCodeMap(
            HashMap::new(),
        ))),
    };
    println!("start server");
    // build our application with some routes
    let app = Router::new()
        .route("/authorization", get(authorization))
        .route("/decide_authorization", post(decide_authorization))
        .route("/debug_store", get(debug_store))
        // グローバルなstoreの代わりとして。
        .layer(Extension(store));

    // run it
    let addr = SocketAddr::from(([127, 0, 0, 1], 3001));
    tracing::debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn debug_store(store: Extension<Store>) -> impl IntoResponse {
    format!("{:?}", store)
}

async fn authorization() -> impl IntoResponse {
    println!("start server");
    let template = AuthorizationTemplate;
    HtmlTemplate(template)
}

/// メアド、パスワードを受け取って、そのユーザー用の token を作る。
async fn decide_authorization(form: Form<Login>, store: Extension<Store>) -> impl IntoResponse {
    println!("start server");
    let Login { email, password } = form.0;
    let requested_user_email = UserEmail(email);

    let mut userEmailMap = &store.userEmailMap;
    let user_email_guard = userEmailMap.try_lock().unwrap();
    let got_user = user_email_guard.0.get(&requested_user_email);

    let user = got_user.unwrap();

    let User {
        password: user_pass,
        id,
        email,
    } = user;

    let redirected = if &password == user_pass {
        // 普通はあらかじめ権限リクエストするアプリを作った人がどこにリダイレクトさせておきたいかを登録している想定
        let redirect_url = "http://localhost:3000/redirected";
        let code = format!("this_is_ninka_code_of_user_id_{}", &user.id.0);

        let mut available_authorization_code_map_guard =
            store.ableableAuthorizationCodeMap.try_lock().unwrap();
        available_authorization_code_map_guard
            .0
            .insert(AuthorizationCode(code.clone()), user.clone());

        let formated = format!("{}?code={}", redirect_url, code);
        let path = formated.as_str();
        Redirect::temporary(path)
    } else {
        Redirect::temporary("404")
    };
    redirected
    // 返り値がなくてもコンパイルエラーにならないのどうにかしたい。
}

#[derive(Template)]
#[template(path = "authorization.html")]
struct AuthorizationTemplate;

struct HtmlTemplate<T>(T);

impl<T> IntoResponse for HtmlTemplate<T>
where
    T: Template,
{
    fn into_response(self) -> Response {
        match self.0.render() {
            Ok(html) => Html(html).into_response(),
            Err(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to render template. Error: {}", err),
            )
                .into_response(),
        }
    }
}
