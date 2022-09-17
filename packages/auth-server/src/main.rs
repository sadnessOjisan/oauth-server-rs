use askama::Template;
use axum::{
    extract::Form,
    headers::HeaderMap,
    http::{header::CONTENT_TYPE, Method, StatusCode},
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
    Extension, Json, Router,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, net::SocketAddr, sync::Arc};
use tokio::sync::Mutex;
use tower_http::cors::CorsLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
#[derive(Deserialize)]
struct Login {
    email: String,
    password: String,
}

#[derive(Deserialize)]
struct TokenEndpoint {
    grant_type: String,
    code: String, // 認可コード
    redirect_uri: Option<String>,
    code_verifier: Option<String>,
}

#[derive(Serialize)]
struct TokenEndopointResponse {
    access_token: String,          // 必須
    token_type: String,            // 必須
    expires_in: Option<u32>,       // 任意
    refresh_token: Option<String>, // 任意
    scope: Option<String>,         // 要求したスコープ群と差異があれば必須
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
    user_email_map: Arc<Mutex<UserEmailMap>>,
    ableable_authorization_code_map: Arc<Mutex<AbleableAuthorizationCodeMap>>,
    ableable_access_token_map: Arc<Mutex<AbleableAccessTokenMap>>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "auth-server=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // 登録済みユーザーをあらかじめ作成しておく
    let mut users = UserEmailMap(HashMap::new());
    let user_email_1 = UserEmail("sadness_ojisan@example.com".to_string());
    let user_1 = User {
        id: UserId(1),
        email: user_email_1.clone(),
        password: "sadness_ojisan".to_string(),
    };
    users.0.insert(user_email_1, user_1);
    let store: Store = Store {
        user_email_map: Arc::new(Mutex::new(users)),
        ableable_access_token_map: Arc::new(Mutex::new(AbleableAccessTokenMap(HashMap::new()))),
        ableable_authorization_code_map: Arc::new(Mutex::new(AbleableAuthorizationCodeMap(
            HashMap::new(),
        ))),
    };
    println!("start server");
    // build our application with some routes
    let cors = CorsLayer::new()
        // allow `GET` and `POST` when accessing the resource
        .allow_methods([Method::GET, Method::POST])
        // allow requests from any origin
        .allow_origin(["http://localhost:3000".parse().unwrap()])
        .allow_credentials(true)
        .allow_headers([CONTENT_TYPE]);
    let app = Router::new()
        .route("/authorization", get(authorization))
        .route("/decide_authorization", post(decide_authorization))
        .route("/debug_store", get(debug_store))
        .route("/token_endpoint", post(token_endpoint))
        .layer(cors)
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
    let template = AuthorizationTemplate;
    HtmlTemplate(template)
}

fn create_access_token(code: &String) -> String {
    format!("access_token_from_{}", code)
}

async fn token_endpoint(
    Json(input): Json<TokenEndpoint>,
    store: Extension<Store>,
) -> impl IntoResponse {
    let TokenEndpoint {
        grant_type: _,
        code,
        code_verifier: _,
        redirect_uri: _,
    } = input;

    let access_token = create_access_token(&code);
    let mut access_token_guard = store.ableable_access_token_map.try_lock().unwrap();
    let authorization_code_map_guard = store.ableable_authorization_code_map.try_lock().unwrap();
    let access_user = authorization_code_map_guard
        .0
        .get(&AuthorizationCode(code))
        .expect("not exist code");
    access_token_guard
        .0
        .insert(AccessToken(access_token.clone()), access_user.clone());
    let mut headers = HeaderMap::new();

    headers.insert(
        "Set-Cookie",
        format!(
            "access_token={};Secure; domain=localhost; Max-Age=60000; SameSite=None",
            &access_token
        )
        .parse()
        .unwrap(),
    );

    headers.insert(
        "Access-Control-Allow-Origin",
        "http://localhost:3000".parse().unwrap(),
    );

    headers.insert("Access-Control-Allow-Credentials", "true".parse().unwrap());

    (
        headers,
        Json(TokenEndopointResponse {
            expires_in: None,
            access_token: access_token,
            token_type: "".to_string(),
            refresh_token: None,
            scope: None,
        }),
    )
}

/// メアド、パスワードを受け取って、そのユーザー用の token を作る。
async fn decide_authorization(form: Form<Login>, store: Extension<Store>) -> impl IntoResponse {
    println!("start server");
    let Login { email, password } = form.0;
    let requested_user_email = UserEmail(email);

    let user_email_map = &store.user_email_map;
    let user_email_guard = user_email_map.try_lock().unwrap();
    let got_user = user_email_guard.0.get(&requested_user_email);

    let user = got_user.unwrap();

    let User {
        password: user_pass,
        id: _,
        email: _,
    } = user;

    let redirected = if &password == user_pass {
        // 普通はあらかじめ権限リクエストするアプリを作った人がどこにリダイレクトさせておきたいかを登録している想定
        let redirect_url = "http://localhost:3000/redirected";
        let code = format!("this_is_ninka_code_of_user_id_{}", &user.id.0);

        let mut available_authorization_code_map_guard =
            store.ableable_authorization_code_map.try_lock().unwrap();
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
