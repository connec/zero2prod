use std::{env, net::Ipv4Addr};

use sqlx::PgPool;
use tracing::{info, Level};
use tracing_subscriber::{
    filter::Targets, layer::SubscriberExt as _, util::SubscriberInitExt as _,
};

const DEFAULT_PORT: u16 = 8000;

#[tokio::main]
async fn main() -> zero2prod::ServerResult {
    let filter = if let Ok(filter) = env::var("RUST_LOG") {
        filter.parse().expect("invalid configuration for RUST_LOG")
    } else {
        Targets::new()
    }
    .with_target(env!("CARGO_PKG_NAME"), Level::DEBUG);

    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(filter)
        .init();

    let config = zero2prod::Config::from_env().expect("failed to read configuration");

    let pool = PgPool::connect_with(config.database)
        .await
        .expect("failed to connect to database");

    let server = zero2prod::bind(
        pool,
        &(Ipv4Addr::LOCALHOST, config.port.unwrap_or(DEFAULT_PORT)).into(),
    );

    info!("Listening on {}", server.local_addr());

    server.await
}
