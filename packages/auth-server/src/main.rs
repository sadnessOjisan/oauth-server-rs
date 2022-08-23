use askama::Template;
use axum::{
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
    routing::get,
    Extension, Router,
};
use std::{collections::HashMap, iter::Map, net::SocketAddr};

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

struct AuthorizationCode(String);

struct AbleableAuthorizationCodeMap(HashMap<AuthorizationCode, User>);

struct AccessToken(String);

struct AbleableAccessTokenMap(HashMap<AccessToken, User>);

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
    println!("start server");
    // build our application with some routes
    let app = Router::new()
        .route("/authorization", get(authorization))
        .route("/decide_authorization", get(decide_authorization))
        .layer(Extension(users));

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
async fn decide_authorization(users: Extension<UserEmailMap>) -> impl IntoResponse {
    let requested_email = "sadness_ojisan@example.com".to_string();
    let requested_pass = "sadness_ojisan";
    let requested_user_email = UserEmail(requested_email);

    let got_user: Option<&User> = users.0.0.get(&requested_user_email);
    let user = got_user.unwrap();
    let user_pass = user.password.as_str();

    // if 文のなかから redirect できないんだっけ？
    let redirected = if requested_pass == user_pass {
        // 普通はあらかじめ権限リクエストするアプリを作った人がどこにリダイレクトさせておきたいかを登録している想定
        let redirect_url = "http://localhost:3000/redirected";
        let code = "this_is_ninka_code";
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
