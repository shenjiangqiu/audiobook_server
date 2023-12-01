use axum::{extract::State, middleware::Next, response::Response};
use hyper::Request;
use tower_cookies::Cookies;
use tracing::debug;

use crate::{
    middleware::{check_passkey, generate_response_util, tools, PasskeyCheckResult},
    AppStat,
};

pub(crate) async fn user_auth<B>(
    State(stats): State<AppStat>,
    request: Request<B>,
    next: Next<B>,
) -> Response {
    debug!("user_auth");
    let cookies: &Cookies = request.extensions().get().unwrap();
    let check_result = check_passkey(cookies, &stats).await;
    match &check_result {
        // if login succeed, then extend the expire time
        PasskeyCheckResult::LogInSucceed((key, _login_info)) => {
            tools::extend_login_expire_time(&stats, &key).await;
        }
        _ => {}
    };

    generate_response_util(request, check_result, |_role_level, request| async move {
        debug!("user is user");
        next.run(request).await
    })
    .await
}
