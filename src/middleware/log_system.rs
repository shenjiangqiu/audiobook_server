use std::net::SocketAddr;

use axum::{extract::ConnectInfo, middleware::Next, response::Response};
use hyper::Request;
use tools::LoginInfo;
use tracing::info;

use crate::middleware::tools;

use super::PasskeyCheckResult;

pub(crate) async fn log_sys<B>(request: Request<B>, next: Next<B>) -> Response {
    let login_info: Option<&LoginInfo> = request.extensions().get();
    let passkey_info: Option<&PasskeyCheckResult> = request.extensions().get();
    let connect_info: &ConnectInfo<SocketAddr> = request.extensions().get().unwrap();
    let addr = connect_info.0.ip();
    match (login_info, passkey_info) {
        (Some(login_info), _) => {
            info!(
                "user {} is accessing,ip: {}, url: {}",
                login_info.user_name,
                addr,
                request.uri().path()
            );
        }
        (None, Some(passkey_result)) => match passkey_result {
            PasskeyCheckResult::NoCookie => {
                info!(
                    "no_login_info(No Cookie), ip: {}, url: {}",
                    addr,
                    request.uri().path()
                )
            }
            PasskeyCheckResult::NoRedis => {
                info!(
                    "no_login_info(No redis), ip: {}, url: {}",
                    addr,
                    request.uri().path()
                )
            }
            PasskeyCheckResult::LogInSucceed((_user_id, login_info)) => {
                info!(
                    "user {} is accessing,ip: {}, url: {}",
                    login_info.user_name,
                    addr,
                    request.uri().path()
                );
            }
        },
        _ => {
            info!("no_login_info, ip: {}, url: {}", addr, request.uri().path());
        }
    };
    next.run(request).await
}
