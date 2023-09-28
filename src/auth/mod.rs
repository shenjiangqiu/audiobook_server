use axum::{extract::State, response::IntoResponse, routing::post, Json};
use cookie::time::Duration;
use cookie::Cookie;

use hyper::{header::LOCATION, HeaderMap, StatusCode};
use redis::AsyncCommands;
use sea_orm::{ActiveValue, ColumnTrait, EntityTrait, QueryFilter};
use tower::ServiceBuilder;
use tower_cookies::Cookies;
use tracing::debug;

use crate::consts::USR_COOKIE_KEY;
use crate::middleware::LoginInfo;
use crate::{entities, AppStat};
use entities::prelude::*;
use entities::*;

#[derive(Debug, serde::Deserialize)]
enum RoleLevel {
    Admin,
    User,
}
#[derive(Debug, serde::Deserialize)]
pub struct UserInfo {
    username: String,
    password: String,
    role_level: RoleLevel,
}

pub(crate) fn route(state: super::AppStat) -> axum::Router<super::AppStat> {
    axum::Router::new()
        .route("/", post(create_account).get(get_account))
        .route_layer(
            ServiceBuilder::new().layer(axum::middleware::from_fn_with_state(
                state,
                super::middleware::admin_auth::admin_auth,
            )),
        )
        .route("/login", post(login))
        .route("/logout", post(logout))
}

#[derive(Debug, serde::Serialize)]
pub struct User {
    id: u32,
    username: String,
}
async fn create_account(
    State(state): State<AppStat>,
    Json(user_info): Json<UserInfo>,
) -> impl IntoResponse {
    debug!("create_account: {:?}", user_info);
    let passwd_md5 = format!("{:x}", md5::compute(&user_info.password));
    let new_account = account::ActiveModel {
        name: ActiveValue::Set(user_info.username),
        password: ActiveValue::Set(passwd_md5),
        role_level: ActiveValue::Set(match user_info.role_level {
            RoleLevel::Admin => 0,
            RoleLevel::User => 1,
        }),
        ..Default::default()
    };
    let account_result = Account::insert(new_account)
        .exec(&state.connections.db)
        .await;
    match account_result {
        Ok(_) => {
            let mut headers = HeaderMap::new();
            headers.insert(LOCATION, "/".parse().unwrap());
            // status 200
            (StatusCode::OK, headers, "create account succeed")
        }
        Err(_) => {
            let mut headers = HeaderMap::new();
            headers.insert(LOCATION, "/".parse().unwrap());
            // status InternalServerError
            (StatusCode::CONFLICT, headers, "create account failed")
        }
    }
}

#[derive(Debug, serde::Serialize)]
struct AccountResponse {
    username: String,
    user_id: i32,
    role_level: i32,
}

async fn get_account(State(state): State<AppStat>) -> Json<Vec<AccountResponse>> {
    let users = Account::find().all(&state.connections.db).await.unwrap();
    Json(
        users
            .into_iter()
            .map(|u| AccountResponse {
                username: u.name,
                user_id: u.id,
                role_level: u.role_level,
            })
            .collect::<Vec<_>>(),
    )
}
#[derive(Debug, serde::Deserialize)]
struct UserLoginInfo {
    username: String,
    password: String,
}
#[derive(Debug, serde::Serialize)]
struct LoginResult {
    code: i32,
    message: String,
}
async fn login(
    State(state): State<AppStat>,
    cookies: Cookies,
    Json(user_info): Json<UserLoginInfo>,
) -> impl IntoResponse {
    debug!("login: {:?}", user_info);
    let user = Account::find()
        .filter(account::Column::Name.eq(&user_info.username))
        .one(&state.connections.db)
        .await
        .unwrap();
    debug!("user: {:?}", user);
    match user {
        Some(user) => {
            let passwd_md5 = format!("{:x}", md5::compute(user_info.password));
            if passwd_md5 == user.password {
                // login succeed
                // generate redis passkey from random 16 Bytes
                let passkey = rand::random::<[u8; 16]>();
                let passkey_str = hex::encode(passkey);
                // save redis
                let mut redis_conn = state.connections.redis.lock().await;
                let redis_value = LoginInfo {
                    user_id: user.id,
                    role_level: user.role_level,
                    user_name: user.name,
                };
                let _: () = redis_conn
                    .set_ex(&passkey_str, redis_value, 3600 * 24 * 7)
                    .await
                    .unwrap();

                // set cookie
                let mut cookie = cookie::Cookie::new(crate::consts::USR_COOKIE_KEY, passkey_str);
                cookie.set_max_age(Duration::days(7));
                cookie.set_path("/");
                cookies.add(cookie);

                //redirect to /
                debug!("login success");
                Json(LoginResult {
                    code: 0,
                    message: "login success".to_string(),
                })
            } else {
                // wrong password
                debug!(
                    "wrong password, required md5: {}, input: {}",
                    user.password, passwd_md5
                );
                let mut headers = HeaderMap::new();
                headers.insert(LOCATION, "/".parse().unwrap());
                Json(LoginResult {
                    code: 1,
                    message: "wrong password".to_string(),
                })
            }
        }
        None => {
            // no such user
            let mut headers = HeaderMap::new();
            headers.insert(LOCATION, "/".parse().unwrap());
            Json(LoginResult {
                code: 2,
                message: "no such user".to_string(),
            })
        }
    }
}

async fn logout(State(state): State<AppStat>, cookies: Cookies) -> impl IntoResponse {
    debug!("logout");
    let passkey = cookies.get(USR_COOKIE_KEY);
    if let Some(passkey) = passkey {
        debug!("deleting passkey: {}", passkey.value());
        let mut redis_conn = state.connections.redis.lock().await;
        let _: () = redis_conn.del(passkey.value()).await.unwrap();
    }
    // delete cookie
    let cookie = Cookie::build(USR_COOKIE_KEY, "").path("/").finish();
    cookies.remove(cookie);
    let mut headers = HeaderMap::new();
    headers.insert(LOCATION, "/".parse().unwrap());
    (headers, "logout success")
}
