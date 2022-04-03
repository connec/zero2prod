use std::net::Ipv4Addr;

use sqlx::PgPool;

const DEFAULT_PORT: u16 = 8000;

#[tokio::main]
async fn main() -> zero2prod::Result {
    let config = zero2prod::Config::from_env().expect("failed to read configuration");

    let pool = PgPool::connect_with(config.database)
        .await
        .expect("failed to connect to database");

    zero2prod::bind(
        pool,
        &(Ipv4Addr::LOCALHOST, config.port.unwrap_or(DEFAULT_PORT)).into(),
    )
    .await
}
