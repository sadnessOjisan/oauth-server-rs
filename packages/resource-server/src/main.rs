use axum::{
    response::{IntoResponse, Json},
    routing::get,
    Extension, Router,
};
use axum_extra::extract::cookie::CookieJar;
use serde::Serialize;
use std::{collections::HashMap, net::SocketAddr, sync::Arc};
use tokio::sync::Mutex;

#[derive(Eq, Hash, PartialEq, Clone, Debug)]
struct UserId(u32);

#[derive(Eq, Hash, PartialEq, Clone, Debug)]
struct UserEmail(String);

#[derive(Eq, Hash, PartialEq, Clone, Debug)]
struct User {
    id: UserId,
    email: UserEmail,
    password: String,
    birth: String,
}

#[derive(Clone, Debug)]
struct UserData(HashMap<UserId, User>);

#[derive(Clone, Debug)]
struct Data {
    user: Arc<Mutex<UserData>>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    // build our application with some routes
    let store = Data {
        user: Arc::new(Mutex::new(UserData(HashMap::new()))),
    };
    let app = Router::new()
        .route("/my_birthday", get(my_birthday))
        .layer(Extension(store));
    // run it
    let addr = SocketAddr::from(([127, 0, 0, 1], 3002));
    tracing::debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

#[derive(Serialize)]
struct BirthDayResponse {
    birth: String,
}

async fn my_birthday(store: Extension<Data>, jar: CookieJar) -> impl IntoResponse {
    let access_token = jar
        .get("access_token")
        .expect("should be exist cookie")
        .value();
    let uid = UserId(1);
    let guard = store.user.try_lock().unwrap();
    let user = guard.0.get(&uid).expect("should be exist");
    Json(BirthDayResponse {
        birth: user.birth.clone(),
    })
}
