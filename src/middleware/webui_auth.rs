use axum::{extract::State, middleware::Next, response::Response};
use hyper::Request;
use tower_cookies::Cookies;
use tracing::debug;

use crate::{
    middleware::{check_passkey, tools, PasskeyCheckResult},
    AppStat,
};

/// just get some infomation from cookie and redis, and provide some infomation to request, do not reject any request
pub(crate) async fn webui_auth<B>(
    State(stats): State<AppStat>,
    mut request: Request<B>,
    next: Next<B>,
) -> Response {
    debug!("webui_auth");
    let cookies: &Cookies = request.extensions().get().unwrap();
    let check_result = check_passkey(cookies, &stats).await;
    match &check_result {
        // if login succeed, then extend the expire time
        PasskeyCheckResult::LogInSucceed((key, _login_info)) => {
            debug!("extend_login_expire_time");
            tools::extend_login_expire_time(&stats, &key).await;
        }
        _ => {}
    };
    request.extensions_mut().insert(check_result);
    next.run(request).await
}
