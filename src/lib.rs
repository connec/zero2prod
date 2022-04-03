use std::net::SocketAddr;

use axum::{http::StatusCode, routing::get, Router};

pub type Server =
    axum::Server<hyper::server::conn::AddrIncoming, axum::routing::IntoMakeService<Router>>;

pub type Result = hyper::Result<()>;

pub fn bind(addr: &SocketAddr) -> Server {
    let app = Router::new().route("/health", get(health));

    axum::Server::bind(addr).serve(app.into_make_service())
}

async fn health() -> StatusCode {
    StatusCode::NO_CONTENT
}
