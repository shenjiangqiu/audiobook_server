use axum::{response::IntoResponse, routing::get};
use clap::Parser;
use hyper::{
    header,
    server::{accept::Accept, conn::AddrIncoming},
    Method, StatusCode,
};
use dotenv::dotenv;
use sea_orm::{Database, DatabaseConnection};
use serde::{Deserialize, Serialize};
#[cfg(not(target_os = "linux"))]
use std::net::Ipv4Addr;
use std::{
    net::{Ipv6Addr, SocketAddr},
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
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
    #[clap(short, long, env = "REDIS_URL", default_value = "redis://10.10.0.2/0")]
    redis: String,

    /// the database url,start at "mysql://"
    #[clap(
        short,
        long,
        env = "DATABASE_URL",
        default_value = "mysql://root:qiuqiu123@10.10.0.2/music_db"
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

    let app = axum::Router::new()
        .nest("/account", auth::route(stat.clone()))
        .nest("/music", music::route(stat.clone()))
        .nest("/progress", progress::route(stat.clone()))
        .nest("/webui", webui::route(stat.clone()))
        .route_layer(tower::ServiceBuilder::new().layer(CookieManagerLayer::new())) // above route need login auth, so need cookie service
        .nest_service("/fetchbook", ServeDir::new(cli.book_dir))
        .route("/", get(|| async { redirect("/webui/index").await }))
        .route(
            "/css/style.css",
            get(|| async {
                (
                    StatusCode::OK,
                    [(header::CONTENT_TYPE, "text/css")],
                    include_str!("../static/css/style.css"),
                )
            }),
        )
        .route(
            "/favicon.ico",
            get(|| async {
                (
                    StatusCode::OK,
                    [(header::CONTENT_TYPE, "image/x-icon")],
                    include_bytes!("../static/favicon.ico"),
                )
            }),
        )
        .fallback_service(ServeDir::new("./static"))
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

struct CombinedAddr {
    a: AddrIncoming,
    b: AddrIncoming,
}
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
    use sea_orm::{Database, EntityTrait};

    use crate::entities;
    use entities::prelude::*;
    use entities::*;
    #[tokio::test]
    async fn test_database() {
        let db = Database::connect("mysql://root:qiuqiu123@10.10.0.2/music_db")
            .await
            .unwrap();
        let account_admin = account::ActiveModel {
            name: sea_orm::ActiveValue::Set("admin".to_string()),
            password: sea_orm::ActiveValue::Set("123".to_string()),
            ..Default::default()
        };
        Account::insert(account_admin).exec(&db).await.unwrap();
    }

    #[tokio::test]
    async fn test_database_get() {
        let db = Database::connect("mysql://root:qiuqiu123@10.10.0.2/music_db")
            .await
            .unwrap();
        let account_admin = Account::find().all(&db).await.unwrap();
        for account in account_admin {
            println!("{:?}", account);
        }
    }
    use redis::AsyncCommands;
    #[tokio::test]
    async fn test_redis() -> eyre::Result<()> {
        let client = redis::Client::open("redis://10.10.0.2/0")?;
        let mut con = client.get_async_connection().await?;
        con.set("hell", "world").await?;
        let hell: String = con.get("hell").await?;
        println!("{}", hell);
        let client = redis::Client::open("redis://10.10.0.2/1")?;
        let mut con = client.get_async_connection().await?;
        con.set("hell2", "world2").await?;
        let hell2: String = con.get("hell2").await?;
        println!("{}", hell2);

        Ok(())
    }
}
