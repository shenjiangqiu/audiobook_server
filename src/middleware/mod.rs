use axum::{
    async_trait, body,
    extract::{FromRequestParts, State},
    http::{self, request::Parts},
    middleware::Next,
    response::{IntoResponse, Response},
};
use futures::Future;
use hyper::{Request, StatusCode};
use redis::{AsyncCommands, FromRedisValue, ToRedisArgs};
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
    generate_response_util(request, check_result, |_, request| async move {
        debug!("user is user");
        next.run(request).await
    })
    .await
}
enum PasskeyCheckResult {
    NoCookie,
    NoRedis,
    /// userid, role_level
    LogInSucceed(LoginInfo),
}

impl FromRedisValue for LoginInfo {
    fn from_redis_value(v: &redis::Value) -> redis::RedisResult<Self> {
        let value = match *v {
            redis::Value::Data(ref bytes) => bytes,
            _ => {
                return Err(redis::RedisError::from((
                    redis::ErrorKind::TypeError,
                    "Response type is not a string",
                )))
            }
        };
        let login_info: Self = bincode::deserialize(value).map_err(|_err| {
            redis::RedisError::from((
                redis::ErrorKind::TypeError,
                "fail to deserialize login info",
            ))
        })?;
        Ok(login_info)
    }
}
impl ToRedisArgs for LoginInfo {
    fn write_redis_args<W>(&self, out: &mut W)
    where
        W: ?Sized + redis::RedisWrite,
    {
        let bytes = bincode::serialize(self).unwrap();
        out.write_arg(bytes.as_slice());
    }
}

async fn check_passkey(cookies: &Cookies, stats: &AppStat) -> PasskeyCheckResult {
    //get cookie passkey from cookie
    let passkey = cookies.get(crate::consts::USR_COOKIE_KEY);
    match passkey {
        Some(cookie) => {
            // check the passkey
            let passkey = cookie.value();
            let mut redis_conn = stats.redis.lock().await;
            let login_info: Result<LoginInfo, redis::RedisError> = redis_conn.get(passkey).await;

            match login_info {
                Ok(login_info) => PasskeyCheckResult::LogInSucceed(login_info),
                Err(_) => {
                    debug!("passkey not found in redis");
                    PasskeyCheckResult::NoRedis
                }
            }
        }
        _ => {
            // return 403, and go to /
            debug!("no cookie found");
            PasskeyCheckResult::NoCookie
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
    check_result: PasskeyCheckResult,
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
        PasskeyCheckResult::LogInSucceed(login_info) => {
            let role_level = login_info.role_level;
            request.extensions_mut().insert(login_info);
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
