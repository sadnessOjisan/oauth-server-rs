use askama::Template;
use axum::{
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
    routing::get,
    Router,
};
use std::net::SocketAddr;

#[tokio::main]
async fn main() {
    println!("start server");
    // build our application with some routes
    let app = Router::new()
        .route("/authorization", get(authorization))
        .route("/decide_authorization", get(decide_authorization));

    // run it
    let addr = SocketAddr::from(([127, 0, 0, 1], 3001));
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
async fn decide_authorization() -> impl IntoResponse {
    // 普通はあらかじめ権限リクエストするアプリを作った人がどこにリダイレクトさせておきたいかを登録している想定
    let redirect_url = "http://localhost:3000/redirected";
    let code = "hgoe";
    let formated = format!("{}?code={}", redirect_url, code);
    let path = formated.as_str();
    Redirect::temporary(path)
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
