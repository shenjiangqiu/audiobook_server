use axum::{middleware::Next, response::Response};
use hyper::{header::COOKIE, Request};

pub async fn user_auth<B>(request: Request<B>, next: Next<B>) -> Response {
    println!("user_auth");
    let cookie = request.headers().get(COOKIE);
    next.run(request).await
}

pub async fn admin_auth<B>(request: Request<B>, next: Next<B>) -> Response {
    println!("admin_auth");
    next.run(request).await
}
