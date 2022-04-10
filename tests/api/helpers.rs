use std::net::{Ipv4Addr, SocketAddr};

use sqlx::{Connection as _, Executor as _};
use uuid::Uuid;

static TRACING_ENABLED: std::sync::Once = std::sync::Once::new();

pub(crate) struct TestApp {
    pub(crate) pool: sqlx::PgPool,
    pub(crate) addr: SocketAddr,
}

impl TestApp {
    pub(crate) async fn spawn() -> Self {
        TRACING_ENABLED.call_once(|| {
            if std::env::var("TEST_LOG").is_ok() {
                zero2prod::telemetry::init("test", std::io::stdout);
            } else {
                zero2prod::telemetry::init("test", std::io::sink);
            }
        });

        let config = zero2prod::Config::builder()
            .address((Ipv4Addr::LOCALHOST, 0).into())
            .email_base_url("http://test".parse().unwrap())
            .email_sender("test@test.test".parse().unwrap())
            .email_authorization_token("foo".to_string())
            .email_send_timeout(std::time::Duration::from_millis(200))
            .build()
            .expect("failed to builder configuration");

        // Create a unique test database
        let database = Uuid::new_v4().to_string();
        create_database(&config.database_options().database("postgres"), &database)
            .await
            .expect("failed to create test database");

        // Set up the app
        let app = zero2prod::App::new(config.with_database(&database));
        let pool = app.pool().clone();
        let server = app.serve().await.expect("failed to serve app");
        let addr = server.local_addr();

        // Run the server in a background task
        tokio::spawn(server);

        Self { pool, addr }
    }
}

async fn create_database(
    options: &sqlx::postgres::PgConnectOptions,
    database: &str,
) -> Result<(), sqlx::Error> {
    let mut connection = sqlx::PgConnection::connect_with(options).await?;

    connection
        .execute(format!(r#"CREATE DATABASE "{}""#, database).as_str())
        .await?;

    Ok(())
}
