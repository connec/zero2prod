use std::net::SocketAddr;

use axum::{
    routing::{get, post},
    Extension, Router,
};
use sqlx::PgPool;
use tower::ServiceBuilder;

use crate::{email_client::EmailClient, routes, telemetry, Error};

pub type Server =
    axum::Server<hyper::server::conn::AddrIncoming, axum::routing::IntoMakeService<Router>>;

pub type ServerResult = hyper::Result<()>;

pub fn bind(pool: PgPool, email_client: EmailClient, addr: &SocketAddr) -> Server {
    let app = Router::new()
        .route("/health", get(routes::health))
        .route("/subscriptions", post(routes::subscribe))
        .layer(
            ServiceBuilder::new()
                .layer(telemetry::id_layer())
                .layer(telemetry::trace_layer())
                .layer(axum_sqlx_tx::Layer::new_with_error::<Error>(pool))
                .layer(Extension(email_client)),
        );

    axum::Server::bind(addr).serve(app.into_make_service())
}
