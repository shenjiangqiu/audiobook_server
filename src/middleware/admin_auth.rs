use axum::{body, extract::State, middleware::Next, response::Response};
use hyper::Request;
use tower_cookies::Cookies;
use tracing::debug;

use crate::{
    middleware::{check_passkey, generate_response_util},
    AppStat,
};

pub(crate) async fn admin_auth<B>(
    State(stats): State<AppStat>,
    request: Request<B>,
    next: Next<B>,
) -> Response {
    debug!("start admin_auth");
    let cookies: &Cookies = request.extensions().get().unwrap();
    let check_result = check_passkey(cookies, &stats).await;
    generate_response_util(request, check_result, |rolid, request| async move {
        if rolid == 0 {
            debug!("user is admin");
            next.run(request).await
        } else {
            debug!("user is not admin");
            // return 403

            Response::builder()
                .status(403)
                .header("Location", "/")
                .body(body::boxed("Only Admin User can operate user".to_string()))
                .unwrap()
        }
    })
    .await
}
