use hyper::server::{accept::Accept, conn::AddrIncoming};
use serde::{Deserialize, Serialize};
use std::{
    net::{Ipv4Addr, Ipv6Addr, SocketAddr},
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};
use tokio::sync::RwLock;

mod account;
mod database;
mod entities;
mod management;
mod middleware;
mod music;

#[derive(Serialize, Deserialize)]
enum MathOp {
    Add,
    Sub,
    Mul,
    Div,
}

type AppStat = Arc<RwLock<u32>>;
#[tokio::main(flavor = "current_thread")]
async fn main() {
    let stat = Arc::new(RwLock::new(0u32));

    let app = axum::Router::new()
        .nest("/account", account::route())
        .nest("/music", music::route())
        .with_state(stat);

    let addr4 = SocketAddr::new(Ipv4Addr::UNSPECIFIED.into(), 3000);
    let addr6 = SocketAddr::new(Ipv6Addr::UNSPECIFIED.into(), 3000);
    let combined = CombinedAddr {
        a: AddrIncoming::bind(&addr4).unwrap(),
        b: AddrIncoming::bind(&addr6).unwrap(),
    };

    axum::Server::builder(combined)
        .serve(app.into_make_service())
        .await
        .unwrap();
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
}
