use axum::{
    async_trait, body,
    extract::{FromRequestParts, State},
    http::{self, request::Parts},
    middleware::Next,
    response::{IntoResponse, Response},
};
use futures::Future;
use hyper::{Request, StatusCode};
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
    generate_response_util(request, &check_result, |_, request| async move {
        debug!("user is user");
        next.run(request).await
    })
    .await
}
enum PasskeyCheckResult {
    NoCookie,
    NoRedis,
    /// userid, role_level
    RoleLevel(i32, i32),
}

async fn check_passkey(cookies: &Cookies, stats: &AppStat) -> PasskeyCheckResult {
    //get cookie passkey from cookie
    let passkey = cookies.get("passkey");
    match passkey {
        Some(cookie) => {
            // check the passkey
            let passkey = cookie.value();
            let mut redis_conn = stats.redis.lock().await;
            let user_id: Result<i32, redis::RedisError> = redis_conn.get(passkey).await;

            match user_id {
                Ok(user_id) => {
                    debug!("passkey found in redis");
                    let role_level: Result<i32, redis::RedisError> = redis_conn.get(user_id).await;
                    match role_level {
                        Ok(level) => PasskeyCheckResult::RoleLevel(user_id, level),
                        Err(_) => {
                            debug!("role_level not found in redis");
                            PasskeyCheckResult::NoRedis
                        }
                    }
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

#[derive(Debug, Clone)]
pub(crate) struct LoginInfo {
    pub user_id: i32,
    pub role_level: i32,
}

#[async_trait]
impl<S> FromRequestParts<S> for LoginInfo
where
    S: Sync + Send,
{
    type Rejection = (http::StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts.extensions.get::<LoginInfo>().cloned().ok_or((
            StatusCode::INTERNAL_SERVER_ERROR,
            "Can't extract LoginInfo. Is `user_auth` or `adim_auth` enabled?",
        ))
    }
}

async fn generate_response_util<T, R, O, B>(
    mut request: Request<B>,
    check_result: &PasskeyCheckResult,
    on_success: T,
) -> Response
where
    T: FnOnce(i32, Request<B>) -> R,
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
        &PasskeyCheckResult::RoleLevel(user_id, role_level) => {
            request.extensions_mut().insert(LoginInfo {
                user_id,
                role_level,
            });
            on_success(role_level, request).await.into_response()
        }
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
    generate_response_util(request, &check_result, |rolid, request| async move {
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
