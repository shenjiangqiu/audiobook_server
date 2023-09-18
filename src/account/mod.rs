use axum::{
    middleware::{self, FromFn},
    response::{AppendHeaders, IntoResponse, Response},
    routing::post,
    Json,
};
use hyper::{header::SET_COOKIE, HeaderMap};
use tower::ServiceBuilder;

#[derive(Debug, serde::Deserialize)]
pub struct UserInfo {
    username: String,
    password: String,
}

pub(crate) fn route() -> axum::Router<super::AppStat> {
    axum::Router::new()
        .route("/", post(create_account).get(get_account))
        .route_layer(
            ServiceBuilder::new().layer(axum::middleware::from_fn(super::middleware::admin_auth)),
        )
        .route("/login", post(login))
        .route("/logout", post(logout))
}

#[derive(Debug, serde::Serialize)]
pub struct User {
    id: u32,
    username: String,
}
pub async fn create_account(user_info: Json<UserInfo>) {
    println!("create_account: {:?}", user_info);
}

pub async fn get_account() -> Json<User> {
    println!("get_account");
    Json(User {
        id: 0,
        username: "test".to_string(),
    })
}

pub async fn login(Json(user_info): Json<UserInfo>) -> impl IntoResponse {
    println!("login: {:?}", user_info);
    let mut headers = HeaderMap::new();
    headers.insert(
        SET_COOKIE,
        "passkey=qiuqiu123;Max-Age=3600;Path=/".parse().unwrap(),
    );
    (headers, "login success")
    //
}

pub async fn logout() -> impl IntoResponse {
    println!("logout");
    let mut headers = HeaderMap::new();
    headers.insert(SET_COOKIE, "passkey=;Max-Age=0;Path=/".parse().unwrap());
    (headers, "logout success")
}
