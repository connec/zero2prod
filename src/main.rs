use std::time::Duration;

use sqlx::postgres::PgPoolOptions;
use tracing::info;

#[tokio::main]
async fn main() -> zero2prod::ServerResult {
    zero2prod::telemetry::init(env!("CARGO_PKG_NAME"), std::io::stdout);

    let config = zero2prod::Config::from_env().expect("failed to read configuration");

    let pool = PgPoolOptions::new()
        .connect_timeout(Duration::from_secs(2))
        .connect_lazy_with(config.database_options());

    let server = zero2prod::bind(pool, &config.addr());

    info!("Listening on {}", server.local_addr());

    server.await
}
