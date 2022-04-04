use std::{
    net::{IpAddr, Ipv4Addr},
    time::Duration,
};

use sqlx::postgres::PgPoolOptions;
use tracing::info;

const DEFAULT_ADDRESS: IpAddr = IpAddr::V4(Ipv4Addr::LOCALHOST);
const DEFAULT_PORT: u16 = 8000;

#[tokio::main]
async fn main() -> zero2prod::ServerResult {
    zero2prod::telemetry::init(env!("CARGO_PKG_NAME"), std::io::stdout);

    let config = zero2prod::Config::from_env().expect("failed to read configuration");

    let pool = PgPoolOptions::new()
        .connect_timeout(Duration::from_secs(2))
        .connect_lazy_with(config.database);

    let server = zero2prod::bind(
        pool,
        &(
            config.address.unwrap_or(DEFAULT_ADDRESS),
            config.port.unwrap_or(DEFAULT_PORT),
        )
            .into(),
    );

    info!("Listening on {}", server.local_addr());

    server.await
}
