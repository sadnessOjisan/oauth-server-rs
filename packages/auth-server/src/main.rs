use askama::Template;
use axum::{
    extract::Form,
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
    Extension, Router,
};
use std::{collections::HashMap, iter::Map, net::SocketAddr, sync::Arc};
use tokio::sync::{Mutex, RwLock};

use serde::Deserialize;

#[derive(Deserialize)]
struct Login {
    email: String,
    password: String,
}

#[derive(Clone)]
struct UserId(u32);

#[derive(Eq, Hash, PartialEq, Clone)]
struct UserEmail(String);

#[derive(Clone)]
struct User {
    id: UserId,
    email: UserEmail,
    password: String,
}

#[derive(Clone)]
struct UserEmailMap(HashMap<UserEmail, User>);

/// 認可コード
#[derive(Eq, Hash, PartialEq, Clone)]
struct AuthorizationCode(String);

#[derive(Clone)]
struct AbleableAuthorizationCodeMap(HashMap<AuthorizationCode, User>);

#[derive(Clone)]
struct AccessToken(String);

#[derive(Clone)]
struct AbleableAccessTokenMap(HashMap<AccessToken, User>);

#[derive(Clone)]
struct Store {
    userEmailMap: UserEmailMap,
    ableableAuthorizationCodeMap: AbleableAuthorizationCodeMap,
    ableableAccessTokenMap: AbleableAccessTokenMap,
}

type SharedStore = Arc<Mutex<Store>>;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let mut users = UserEmailMap(HashMap::new());
    let user_email_1 = UserEmail("sadness_ojisan@example.com".to_string());
    let user_1 = User {
        id: UserId(1),
        email: user_email_1.clone(),
        password: "sadness_ojisan".to_string(),
    };
    users.0.insert(user_email_1, user_1);
    let store: SharedStore = Arc::new(Mutex::new(Store {
        userEmailMap: users,
        ableableAccessTokenMap: AbleableAccessTokenMap(HashMap::new()),
        ableableAuthorizationCodeMap: AbleableAuthorizationCodeMap(HashMap::new()),
    }));
    println!("start server");
    // build our application with some routes
    let app = Router::new()
        .route("/authorization", get(authorization))
        .route("/decide_authorization", post(decide_authorization))
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

async fn authorization() -> impl IntoResponse {
    let template = AuthorizationTemplate;
    HtmlTemplate(template)
}

/// メアド、パスワードを受け取って、そのユーザー用の token を作る。
async fn decide_authorization(
    form: Form<Login>,
    store: Extension<SharedStore>,
) -> impl IntoResponse {
    let requested_email = &form.email;
    let requested_pass = &form.password;
    let requested_user_email = UserEmail(requested_email.to_string());

    let mut locked_store = store.0.try_lock().unwrap();
    let got_user = &locked_store.userEmailMap.0.get(&requested_user_email);
    let user = got_user.unwrap().clone();
    let user_pass = user.password.as_str();

    let redirected = if requested_pass == user_pass {
        // 普通はあらかじめ権限リクエストするアプリを作った人がどこにリダイレクトさせておきたいかを登録している想定
        let redirect_url = "http://localhost:3000/redirected";
        let code = format!("this_is_ninka_code_of_user_id_{}", &user.id.0);

        // drop できなくてここのロックを剥がせない。どうすれば？
        locked_store
            .ableableAuthorizationCodeMap
            .0
            .insert(AuthorizationCode(code.clone()), user.clone());

        let formated = format!("{}?code={}", redirect_url, code);
        let path = formated.as_str();
        Redirect::temporary(path)
    } else {
        // 404 を返したいが、if-else で同じ型を強制されてしまう
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
