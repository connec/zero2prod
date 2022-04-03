use std::net::SocketAddr;

use axum::{
    routing::{get, post},
    Router,
};
use sqlx::PgPool;

use crate::routes;

pub type Server =
    axum::Server<hyper::server::conn::AddrIncoming, axum::routing::IntoMakeService<Router>>;

pub type Result = hyper::Result<()>;

pub fn bind(pool: PgPool, addr: &SocketAddr) -> Server {
    let app = Router::new()
        .route("/health", get(routes::health))
        .route("/subscriptions", post(routes::subscribe))
        .layer(axum_sqlx_tx::Layer::new(pool));

    axum::Server::bind(addr).serve(app.into_make_service())
}
