use std::time::Duration;

use sqlx::postgres::PgPoolOptions;
use tracing::{info, info_span};
use zero2prod::EmailClient;

#[tokio::main]
async fn main() -> zero2prod::ServerResult {
    zero2prod::telemetry::init(env!("CARGO_PKG_NAME"), std::io::stdout);

    let config = zero2prod::Config::from_env().expect("failed to read configuration");

    let pool = PgPoolOptions::new()
        .connect_timeout(Duration::from_secs(2))
        .connect_lazy_with(config.database_options());

    {
        let _guard = info_span!("migrate").entered();
        info!("Performing migrations");
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("failed to migrate the database");
        info!("Finished migrations");
    }

    let email_client = EmailClient::new(
        config.email_base_url().clone(),
        config.email_sender().clone(),
        config.email_authorization_token().to_owned(),
        config.email_send_timeout(),
    );

    let server = zero2prod::bind(pool, email_client, &config.addr());

    info!("Listening on {}", server.local_addr());

    server.await
}
