use axum::routing::get;

pub fn route<S>() -> axum::Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    axum::Router::new()
        .route("/list", get(list_book))
        .route("/get", get(get_music))
        .route_layer(
            tower::ServiceBuilder::new()
                .layer(axum::middleware::from_fn(super::middleware::user_auth)),
        )
}

async fn list_book() {
    println!("list book");
}
async fn get_music() {
    println!("get music")
}
