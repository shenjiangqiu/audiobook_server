use axum::{extract::State, middleware::Next, response::Response};
use hyper::Request;
use tower_cookies::Cookies;
use tracing::debug;

use crate::{middleware::check_passkey, AppStat};

/// just get some infomation from cookie and redis, and provide some infomation to request, do not reject any request
pub(crate) async fn webui_auth<B>(
    State(stats): State<AppStat>,
    mut request: Request<B>,
    next: Next<B>,
) -> Response {
    debug!("webui_auth");
    let cookies: &Cookies = request.extensions().get().unwrap();
    let check_result = check_passkey(cookies, &stats).await;
    request.extensions_mut().insert(check_result);
    next.run(request).await
}
