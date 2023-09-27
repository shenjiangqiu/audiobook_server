use axum::headers::IfModifiedSince;
use axum::response::Response;
use axum::TypedHeader;
use axum::{response::IntoResponse, routing::get, Router};
use clap::Parser;
use dotenv::dotenv;
#[cfg(not(target_os = "linux"))]
use hyper::server::{accept::Accept, conn::AddrIncoming};
use hyper::{header, Method, StatusCode};
use lazy_static::lazy_static;
use mime::Mime;
use sea_orm::{Database, DatabaseConnection};
use serde::{Deserialize, Serialize};
use std::env;
#[cfg(not(target_os = "linux"))]
use std::net::Ipv4Addr;
#[cfg(not(target_os = "linux"))]
use std::pin::Pin;
use std::str::FromStr;
#[cfg(not(target_os = "linux"))]
use std::task::{Context, Poll};

use std::time::SystemTime;
use std::{
    net::{Ipv6Addr, SocketAddr},
    sync::Arc,
};
use tera::Tera;
use tokio::sync::Mutex;
use tower_cookies::CookieManagerLayer;
use tower_http::{
    cors::{Any, CorsLayer},
    services::ServeDir,
};
use tracing::{debug, info};

mod auth;
pub mod consts;
mod database;
pub mod entities;
mod management;
mod middleware;
mod music;
pub(crate) mod progress;
pub mod tools;
mod webui;

lazy_static! {
    pub static ref BUILD_TIME: SystemTime = {
        let build_sec: i64 = env!("BUILD_TIME").parse().unwrap();
        SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(build_sec as u64)
    };
}

#[derive(Serialize, Deserialize)]
enum MathOp {
    Add,
    Sub,
    Mul,
    Div,
}

pub(crate) struct AppStats {
    pub tera: Tera,
    pub connections: AppConnections,
}
pub(crate) struct AppConnections {
    pub db: DatabaseConnection,
    pub redis: Mutex<redis::aio::Connection>,
}
impl AppConnections {
    pub fn new(db: DatabaseConnection, redis: redis::aio::Connection) -> Self {
        Self {
            db,
            redis: Mutex::new(redis),
        }
    }
}
type AppStat = Arc<AppStats>;
#[derive(Debug, Parser)]
pub struct Cli {
    /// the redis url,start at "redis://"
    #[clap(short, long, env = "REDIS_URL", default_value = "redis://localhost/0")]
    redis: String,

    /// the database url,start at "mysql://"
    #[clap(
        short,
        long,
        env = "DATABASE_URL",
        default_value = "mysql://root:qiuqiu123@localhost/music_db"
    )]
    db: String,

    #[clap(short, long, env = "PORT", default_value = "3000")]
    port: u16,
    /// the path store all books
    #[clap(short, long, env = "BOOKS", default_value = "./books")]
    book_dir: String,
}

pub fn init_log() {
    // init tracing_subscriber
    tracing_subscriber::fmt::SubscriberBuilder::default()
        .with_env_filter(
            tracing_subscriber::EnvFilter::builder()
                .with_default_directive("audiobook_server=info".parse().unwrap())
                .from_env_lossy(),
        )
        .with_ansi(true)
        .init();
}
pub async fn init_mysql(db: &str) -> DatabaseConnection {
    let db = Database::connect(db).await.unwrap();
    db
}
pub async fn init_redis(redis: &str) -> redis::aio::Connection {
    let redis = redis::Client::open(redis)
        .unwrap()
        .get_async_connection()
        .await
        .unwrap();
    redis
}

pub async fn init_db(db: &str, redis: &str) -> (DatabaseConnection, redis::aio::Connection) {
    (init_mysql(db).await, init_redis(redis).await)
}

async fn redirect(redirect_path: &str) -> impl IntoResponse {
    // redirect to redirect_path
    (
        StatusCode::FOUND,
        [(header::LOCATION, redirect_path.to_owned())],
    )
}
fn setup_tera() -> Tera {
    let mut tera = Tera::default();
    let index = include_str!("../templates/index.tera");
    let login = include_str!("../templates/login.tera");
    let logout = include_str!("../templates/logout.tera");
    let author_detail = include_str!("../templates/author_detail.tera");
    let book_detail = include_str!("../templates/book_detail.tera");
    let books = include_str!("../templates/books.tera");
    let authors = include_str!("../templates/authors.tera");
    let base = include_str!("../templates/base.tera");
    let player = include_str!("../templates/player.tera");
    let newplayer = include_str!("../templates/newplayer.tera");
    let manager = include_str!("../templates/manager.tera");
    let book_manager = include_str!("../templates/book_manager.tera");
    let account_manager = include_str!("../templates/account_manager.tera");
    let manager_base = include_str!("../templates/manager_base.tera");
    tera.add_raw_templates([
        ("index.tera", index),
        ("login.tera", login),
        ("logout.tera", logout),
        ("author_detail.tera", author_detail),
        ("book_detail.tera", book_detail),
        ("books.tera", books),
        ("authors.tera", authors),
        ("base.tera", base),
        ("player.tera", player),
        ("newplayer.tera", newplayer),
        ("manager.tera", manager),
        ("book_manager.tera", book_manager),
        ("account_manager.tera", account_manager),
        ("manager_base.tera", manager_base),
    ])
    .unwrap();
    tera
}

pub async fn app_main() -> eyre::Result<()> {
    dotenv().ok();
    init_log();
    let cli = Cli::parse();
    debug!("cli:{:?}", cli);

    info!("redis url:{}", cli.redis);
    info!("database url:{}", cli.db);
    info!("starting server,connecting to database and redis");

    info!("database connected");
    let (db, redis) = init_db(&cli.db, &cli.redis).await;
    let stat: AppStat = Arc::new(AppStats {
        tera: setup_tera(),
        connections: AppConnections::new(db, redis),
    });
    let fetch_book_router = Router::new()
        .nest_service("/fetchbook", ServeDir::new(cli.book_dir))
        .route_layer(axum::middleware::from_fn_with_state(
            stat.clone(),
            middleware::user_auth::user_auth,
        ));

    let app = Router::new()
        .nest("/account", auth::route(stat.clone()))
        .nest("/music", music::route(stat.clone()))
        .nest("/progress", progress::route(stat.clone()))
        .nest("/webui", webui::route(stat.clone()))
        .merge(fetch_book_router)
        .route_layer(CookieManagerLayer::new()) // above route need login auth, so need cookie service
        .route("/", get(|| async { redirect("/webui/index").await }))
        .route(
            "/css/style.css",
            get(
                |if_last_modified: Option<TypedHeader<axum::headers::IfModifiedSince>>| async move {
                    let text = include_bytes!("../static/css/style.css");
                    let text_type = TypedHeader(axum::headers::ContentType::from(
                        Mime::from_str("text/css").unwrap(),
                    ));

                    cached_response(if_last_modified, text_type, text.to_vec())
                },
            ),
        )
        .route(
            "/favicon.ico",
            get(
                |if_last_modified: Option<TypedHeader<axum::headers::IfModifiedSince>>| async move {
                    let text = include_bytes!("../static/favicon.ico");
                    let text_type = TypedHeader(axum::headers::ContentType::from(
                        Mime::from_str("image/x-icon").unwrap(),
                    ));
                    cached_response(if_last_modified, text_type, text.to_vec())
                },
            ),
        )
        .route_layer(
            CorsLayer::new()
                .allow_methods([Method::GET, Method::POST])
                .allow_origin(Any),
        )
        .with_state(stat);
    #[cfg(target_os = "linux")]
    {
        let addr6 = SocketAddr::new(Ipv6Addr::UNSPECIFIED.into(), cli.port);
        axum::Server::bind(&addr6)
            .serve(app.into_make_service())
            .await
            .unwrap();
    }
    #[cfg(not(target_os = "linux"))]
    {
        let addr4 = SocketAddr::new(Ipv4Addr::UNSPECIFIED.into(), cli.port);
        let addr6 = SocketAddr::new(Ipv6Addr::UNSPECIFIED.into(), cli.port);
        let combined = CombinedAddr {
            a: AddrIncoming::bind(&addr4).unwrap(),
            b: AddrIncoming::bind(&addr6).unwrap(),
        };
        info!("server started at addrv4: {}", addr4);
        info!("server started at addrv6: {}", addr6);
        axum::Server::builder(combined)
            .serve(app.into_make_service())
            .await
            .unwrap();
    }

    Ok(())
}

fn cached_response(
    if_last_modified: Option<TypedHeader<IfModifiedSince>>,
    text_type: TypedHeader<axum::headers::ContentType>,
    text: Vec<u8>,
) -> Response {
    match if_last_modified {
        Some(if_last_modified) => {
            debug!("if_last_modified:{:?}", if_last_modified);
            debug!("BUILD_TIME:{:?}", *BUILD_TIME);
            if if_last_modified.is_modified(*BUILD_TIME) {
                (
                    StatusCode::OK,
                    text_type,
                    TypedHeader(axum::headers::LastModified::from(*BUILD_TIME)),
                    text,
                )
                    .into_response()
            } else {
                // no modified
                debug!("no modified");
                (StatusCode::NOT_MODIFIED, "").into_response()
            }
        }
        None => (
            StatusCode::OK,
            text_type,
            TypedHeader(axum::headers::LastModified::from(*BUILD_TIME)),
            text,
        )
            .into_response(),
    }
}
#[cfg(not(target_os = "linux"))]

struct CombinedAddr {
    a: AddrIncoming,
    b: AddrIncoming,
}

#[cfg(not(target_os = "linux"))]

impl Accept for CombinedAddr {
    type Conn = <AddrIncoming as Accept>::Conn;
    type Error = <AddrIncoming as Accept>::Error;
    fn poll_accept(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Self::Conn, Self::Error>>> {
        if let Poll::Ready(v) = Pin::new(&mut self.a).poll_accept(cx) {
            return Poll::Ready(v);
        }
        if let Poll::Ready(v) = Pin::new(&mut self.b).poll_accept(cx) {
            return Poll::Ready(v);
        }
        Poll::Pending
    }
}

#[cfg(test)]
mod tests {
    use chrono::format::StrftimeItems;
    use chrono::{DateTime, FixedOffset, Utc};
    use sea_orm::{Database, EntityTrait};

    use crate::entities;
    use entities::prelude::*;
    use entities::*;

    #[tokio::test]
    async fn test_database() {
        // let db = Database::connect("mysql://root:qiuqiu123@localhost/music_db")
        //     .await
        //     .unwrap();
        // let account_admin = account::ActiveModel {
        //     name: sea_orm::ActiveValue::Set("admin".to_string()),
        //     password: sea_orm::ActiveValue::Set("123".to_string()),
        //     ..Default::default()
        // };
        // Account::insert(account_admin).exec(&db).await.unwrap();
    }

    #[tokio::test]
    async fn test_database_get() {
        // let db = Database::connect("mysql://root:qiuqiu123@localhost/music_db")
        //     .await
        //     .unwrap();
        // let account_admin = Account::find().all(&db).await.unwrap();
        // for account in account_admin {
        //     println!("{:?}", account);
        // }
    }
    use redis::AsyncCommands;
    #[tokio::test]
    async fn test_redis() -> eyre::Result<()> {
        // let client = redis::Client::open("redis://localhost/0")?;
        // let mut con = client.get_async_connection().await?;
        // con.set("hell", "world").await?;
        // let hell: String = con.get("hell").await?;
        // println!("{}", hell);
        // let client = redis::Client::open("redis://localhost/1")?;
        // let mut con = client.get_async_connection().await?;
        // con.set("hell2", "world2").await?;
        // let hell2: String = con.get("hell2").await?;
        // println!("{}", hell2);

        Ok(())
    }

    #[test]
    fn test_build_time() {
        let now: DateTime<Utc> = Utc::now();
        let offset = FixedOffset::east_opt(0).unwrap(); // GMT
        let now = now.with_timezone(&offset);
        let items = StrftimeItems::new("%a, %d %b %Y %H:%M:%S GMT"); // Define the timestamp format
        let formatted_date = now.format_with_items(items).to_string();
        println!("{}", formatted_date);
    }
}
