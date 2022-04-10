use std::net::SocketAddr;

use sqlx::{Connection as _, Executor as _};
use uuid::Uuid;

static TRACING_ENABLED: std::sync::Once = std::sync::Once::new();

pub(crate) struct TestApp {
    pub(crate) pool: sqlx::PgPool,
    pub(crate) addr: SocketAddr,
}

pub(crate) async fn spawn_app() -> TestApp {
    TRACING_ENABLED.call_once(|| {
        if std::env::var("TEST_LOG").is_ok() {
            zero2prod::telemetry::init("test", std::io::stdout);
        } else {
            zero2prod::telemetry::init("test", std::io::sink);
        }
    });

    let env = std::env::vars().chain([
        ("address".to_string(), "127.0.0.1".to_string()),
        ("port".to_string(), "0".to_string()),
        ("email_base_url".to_string(), "http://test".to_string()),
        ("email_sender".to_string(), "test@test.test".to_string()),
        ("email_authorization_token".to_string(), "foo".to_string()),
        ("email_send_timeout_ms".to_string(), "200".to_string()),
    ]);
    let config = zero2prod::Config::from_iter(env).expect("failed to load configuration");

    // Create a unique test database
    let database = Uuid::new_v4().to_string();
    create_database(&config.database_options().database("postgres"), &database)
        .await
        .expect("failed to create test database");

    // Set up the app
    let app = zero2prod::App::new(&config.with_database(&database));
    let pool = app.pool().clone();
    let server = app.serve().await.expect("failed to serve app");
    let addr = server.local_addr();

    // Run the server in a background task
    tokio::spawn(server);

    TestApp { pool, addr }
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
