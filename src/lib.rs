use std::net::SocketAddr;

use axum::{
    extract::Form,
    http::StatusCode,
    routing::{get, post},
    Router,
};

pub type Server =
    axum::Server<hyper::server::conn::AddrIncoming, axum::routing::IntoMakeService<Router>>;

pub type Result = hyper::Result<()>;

pub fn bind(addr: &SocketAddr) -> Server {
    let app = Router::new()
        .route("/health", get(health))
        .route("/subscriptions", post(subscribe));

    axum::Server::bind(addr).serve(app.into_make_service())
}

async fn health() -> StatusCode {
    StatusCode::NO_CONTENT
}

#[derive(serde::Deserialize)]
struct Subscriber {
    name: String,
    email: String,
}

async fn subscribe(Form(form): Form<Subscriber>) -> StatusCode {
    StatusCode::OK
}
