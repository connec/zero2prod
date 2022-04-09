use std::net::{Ipv4Addr, SocketAddr};

use axum::http::StatusCode;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use uuid::Uuid;

use zero2prod::{Config, EmailClient};

#[tokio::test]
async fn health_works() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    let response = client
        .get(format!("http://{}/health", app.addr))
        .send()
        .await
        .expect("failed to execute request");

    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}

#[tokio::test]
async fn subscribe_returns_a_200_for_valid_form_data() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
    let response = client
        .post(format!("http://{}/subscriptions", app.addr))
        .header("content-type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("failed to execute request");

    assert_status("valid", StatusCode::OK, response).await;
    let saved = sqlx::query!("SELECT email, name FROM subscriptions")
        .fetch_one(&app.pool)
        .await
        .expect("failed to fetch saved subscription");
    assert_eq!(saved.name, "le guin");
    assert_eq!(saved.email, "ursula_le_guin@gmail.com");
}

#[tokio::test]
async fn subscribe_returns_a_422_when_data_is_missing() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();
    let bodies = vec![
        ("missing the email", "name=le%20guin"),
        ("missing the name", "email=ursula_le_guin%40gmail.com"),
        ("missing both name and email", ""),
    ];

    for (problem, body) in bodies {
        let response = client
            .post(format!("http://{}/subscriptions", app.addr))
            .header("content-type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("failed to execute request");

        assert_status(problem, StatusCode::UNPROCESSABLE_ENTITY, response).await;
    }
}

#[tokio::test]
async fn subscribe_returns_a_422_when_fields_are_present_but_invalid() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();
    let bodies = vec![
        ("empty name", "name=&email=ursula_le_guin%40gmail.com"),
        ("empty email", "name=Ursula&email="),
        ("invalid email", "name=Ursula&email=definitely-not-an-email"),
    ];

    for (problem, body) in bodies {
        let response = client
            .post(format!("http://{}/subscriptions", app.addr))
            .header("content-type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("failed to execute request");

        assert_status(problem, StatusCode::UNPROCESSABLE_ENTITY, response).await;
    }
}

static TRACING_ENABLED: std::sync::Once = std::sync::Once::new();

struct TestApp {
    pool: PgPool,
    addr: SocketAddr,
}

async fn spawn_app() -> TestApp {
    TRACING_ENABLED.call_once(|| {
        if std::env::var("TEST_LOG").is_ok() {
            zero2prod::telemetry::init("test", std::io::stdout);
        } else {
            zero2prod::telemetry::init("test", std::io::sink);
        }
    });

    let env = std::env::vars().chain([
        ("email_base_url".to_string(), "http://test".to_string()),
        ("email_sender".to_string(), "test@test.test".to_string()),
        ("email_authorization_token".to_string(), "foo".to_string()),
        ("email_send_timeout_ms".to_string(), "200".to_string()),
    ]);
    let config = Config::from_iter(env).expect("failed to load configuration");
    let pool = prepare_db(&config).await;
    let email_client = EmailClient::new(
        config.email_base_url().clone(),
        config.email_sender().clone(),
        config.email_authorization_token().to_owned(),
        config.email_send_timeout(),
    );

    let server = zero2prod::bind(pool.clone(), email_client, &(Ipv4Addr::LOCALHOST, 0).into());
    let addr = server.local_addr();

    tokio::spawn(server);

    TestApp { pool, addr }
}

async fn prepare_db(config: &Config) -> PgPool {
    let mut connection =
        PgConnection::connect_with(&config.database_options_with_database("postgres"))
            .await
            .expect("failed to connect to `postgres` database");

    let database = Uuid::new_v4().to_string();
    connection
        .execute(format!(r#"CREATE DATABASE "{}""#, database).as_str())
        .await
        .expect("failed to create database");

    let pool = PgPool::connect_with(config.database_options_with_database(&database))
        .await
        .expect("failed to connect to test database");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("failed to migrate the database");

    pool
}

async fn assert_status(problem: &str, expected: StatusCode, response: reqwest::Response) {
    assert_eq!(
        expected,
        response.status(),
        "did not get {} when the payload was {problem}
got {}: {:?}",
        expected,
        response.status(),
        response.text().await,
    );
}
