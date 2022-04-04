use std::net::SocketAddr;

use axum::{
    routing::{get, post},
    Router,
};
use sqlx::PgPool;
use tower::ServiceBuilder;

use crate::{request_tracing, routes, Error};

pub type Server =
    axum::Server<hyper::server::conn::AddrIncoming, axum::routing::IntoMakeService<Router>>;

pub type ServerResult = hyper::Result<()>;

pub fn bind(pool: PgPool, addr: &SocketAddr) -> Server {
    let app = Router::new()
        .route("/health", get(routes::health))
        .route("/subscriptions", post(routes::subscribe))
        .layer(
            ServiceBuilder::new()
                .layer(request_tracing::id_layer())
                .layer(request_tracing::trace_layer())
                .layer(axum_sqlx_tx::Layer::new_with_error::<Error>(pool)),
        );

    axum::Server::bind(addr).serve(app.into_make_service())
}
