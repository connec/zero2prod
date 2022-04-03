use std::net::{Ipv4Addr, SocketAddr};

use sqlx::{postgres::PgConnectOptions, Connection, Executor, PgConnection, PgPool};
use uuid::Uuid;
use zero2prod::Config;

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

    assert_eq!(200, response.status().as_u16());
    let saved = sqlx::query!("SELECT email, name FROM subscriptions")
        .fetch_one(&app.pool)
        .await
        .expect("failed to fetch saved subscription");
    assert_eq!(saved.name, "le guin");
    assert_eq!(saved.email, "ursula_le_guin@gmail.com");
}

#[tokio::test]
async fn subscribe_returns_a_400_when_data_is_missing() {
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

        assert_eq!(
            422,
            response.status().as_u16(),
            "did not get 422 Unprocessable Entity when the payload was {problem}
got: {:?}",
            response.text().await,
        );
    }
}

struct TestApp {
    pool: PgPool,
    addr: SocketAddr,
}

async fn spawn_app() -> TestApp {
    let config = Config::from_env().expect("failed to load configuration");
    let pool = prepare_db(config.database).await;

    let server = zero2prod::bind(pool.clone(), &(Ipv4Addr::LOCALHOST, 0).into());
    let addr = server.local_addr();

    tokio::spawn(server);

    TestApp { pool, addr }
}

async fn prepare_db(base_config: PgConnectOptions) -> PgPool {
    let postgres_config = base_config.database("postgres");
    let mut connection = PgConnection::connect_with(&postgres_config)
        .await
        .expect("failed to connect to `postgres` database");

    let database = Uuid::new_v4();
    connection
        .execute(format!(r#"CREATE DATABASE "{}""#, database).as_str())
        .await
        .expect("failed to create database");

    let test_config = postgres_config.database(database.to_string().as_str());
    let pool = PgPool::connect_with(test_config)
        .await
        .expect("failed to connect to test database");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("failed to migrate the database");

    pool
}
