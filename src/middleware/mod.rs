use axum::{
    body,
    extract::State,
    middleware::Next,
    response::{IntoResponse, Response},
};
use futures::Future;
use hyper::Request;
use redis::AsyncCommands;
use tower_cookies::Cookies;
use tracing::debug;

use crate::AppStat;

pub(crate) async fn user_auth<B>(
    State(stats): State<AppStat>,
    request: Request<B>,
    next: Next<B>,
) -> Response {
    debug!("user_auth");
    let cookies: &Cookies = request.extensions().get().unwrap();
    let check_result = check_passkey(cookies, &stats).await;
    generate_response_util(&check_result, |_| async move {
        debug!("user is admin");
        next.run(request).await
    })
    .await
}
enum PasskeyCheckResult {
    NoCookie,
    NoRedis,
    RoleLevel(i32),
}

async fn check_passkey(cookies: &Cookies, stats: &AppStat) -> PasskeyCheckResult {
    //get cookie passkey from cookie
    let passkey = cookies.get("passkey");
    match passkey {
        Some(cookie) => {
            // check the passkey
            let passkey = cookie.value();
            let level: Result<i32, redis::RedisError> = stats.redis.lock().await.get(passkey).await;
            match level {
                Ok(level) => {
                    debug!("passkey found in redis");
                    PasskeyCheckResult::RoleLevel(level)
                }
                Err(_) => {
                    debug!("passkey not found in redis");
                    PasskeyCheckResult::NoRedis
                }
            }
        }
        None => {
            // return 403, and go to /
            debug!("no cookie found");
            PasskeyCheckResult::NoCookie
        }
    }
}

async fn generate_response_util<T, R, O>(
    check_result: &PasskeyCheckResult,
    on_success: T,
) -> Response
where
    T: FnOnce(i32) -> R,
    R: Future<Output = O>,
    O: IntoResponse,
{
    match check_result {
        PasskeyCheckResult::NoCookie => Response::builder()
            .status(403)
            .header("Location", "/")
            .body(body::boxed("Not Login".to_string()))
            .unwrap(),
        PasskeyCheckResult::NoRedis => {
            // return 403
            Response::builder()
                .status(403)
                .header("Location", "/")
                .body(body::boxed("Not Login".to_string()))
                .unwrap()
        }
        PasskeyCheckResult::RoleLevel(role_level) => on_success(*role_level).await.into_response(),
    }
}

pub(crate) async fn admin_auth<B>(
    State(stats): State<AppStat>,
    request: Request<B>,
    next: Next<B>,
) -> Response {
    debug!("start admin_auth");
    let cookies: &Cookies = request.extensions().get().unwrap();
    let check_result = check_passkey(cookies, &stats).await;
    generate_response_util(&check_result, |rolid| async move {
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
