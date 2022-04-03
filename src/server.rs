use std::net::SocketAddr;

use axum::{
    routing::{get, post},
    Router,
};

use crate::routes;

pub type Server =
    axum::Server<hyper::server::conn::AddrIncoming, axum::routing::IntoMakeService<Router>>;

pub type Result = hyper::Result<()>;

pub fn bind(addr: &SocketAddr) -> Server {
    let app = Router::new()
        .route("/health", get(routes::health))
        .route("/subscriptions", post(routes::subscribe));

    axum::Server::bind(addr).serve(app.into_make_service())
}
