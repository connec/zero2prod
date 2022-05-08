use std::{env, net::Ipv4Addr};

use argon2::{password_hash::SaltString, Argon2, Params, PasswordHasher};
use reqwest::Url;
use sqlx::{Connection as _, Executor as _};
use uuid::Uuid;
use wiremock::MockServer;

static TRACING_ENABLED: std::sync::Once = std::sync::Once::new();

pub(crate) struct TestApp {
    pub(crate) port: u16,
    pub(crate) pool: sqlx::PgPool,
    pub(crate) base_url: Url,
    pub(crate) email_server: MockServer,
    pub(crate) test_user: TestUser,
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

        let email_server = MockServer::start().await;

        let config = zero2prod::Config::builder()
            .address((Ipv4Addr::LOCALHOST, 0).into())
            // FIXME: we don't know what address to use 😭
            .base_url("http://127.0.0.1:0".parse().unwrap())
            .database_options(env::var("DATABASE_URL").unwrap().parse().unwrap())
            .email_base_url(email_server.uri().parse().unwrap())
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

        let test_user = TestUser::generate();
        test_user.store(&pool).await;

        Self {
            port: addr.port(),
            pool,
            base_url: format!("http://{}/", addr).parse().unwrap(),
            email_server,
            test_user,
        }
    }

    pub(crate) async fn post_subscriptions(&self, body: impl Into<String>) -> reqwest::Response {
        reqwest::Client::new()
            .post(self.base_url.join("/subscriptions").unwrap())
            .header("content-type", "application/x-www-form-urlencoded")
            .body(body.into())
            .send()
            .await
            .expect("failed to execute request")
    }

    pub(crate) async fn post_newsletters(&self, body: &serde_json::Value) -> reqwest::Response {
        reqwest::Client::new()
            .post(self.base_url.join("/newsletters").unwrap())
            .basic_auth(&self.test_user.username, Some(&self.test_user.password))
            .json(body)
            .send()
            .await
            .expect("failed to execute request")
    }

    pub(crate) fn get_confirmation_links(&self, request: &wiremock::Request) -> ConfirmationLinks {
        let body: serde_json::Value = serde_json::from_slice(&request.body).unwrap();

        let get_link = |s| {
            let links: Vec<_> = linkify::LinkFinder::new()
                .links(s)
                .filter(|link| link.kind() == &linkify::LinkKind::Url)
                .collect();
            assert_eq!(links.len(), 1);

            let mut link: Url = links[0].as_str().parse().unwrap();
            assert_eq!(link.host_str().unwrap(), Ipv4Addr::LOCALHOST.to_string());

            // FIXME: we should ideally inject the correct port into base_url somehow
            link.set_port(Some(self.port)).unwrap();

            link
        };

        let html = get_link(body["HtmlBody"].as_str().unwrap());
        let text = get_link(body["HtmlBody"].as_str().unwrap());
        ConfirmationLinks { html, text }
    }
}

pub(crate) struct TestUser {
    pub(crate) id: Uuid,
    pub(crate) username: String,
    pub(crate) password: String,
}

impl TestUser {
    pub(crate) fn generate() -> Self {
        Self {
            id: Uuid::new_v4(),
            username: Uuid::new_v4().to_string(),
            password: Uuid::new_v4().to_string(),
        }
    }

    async fn store(&self, pool: &sqlx::PgPool) {
        let salt = SaltString::generate(&mut rand::thread_rng());
        let password_hash = Argon2::new(
            argon2::Algorithm::Argon2id,
            argon2::Version::V0x13,
            Params::new(15_000, 2, 1, None).unwrap(),
        )
        .hash_password(self.password.as_bytes(), &salt)
        .unwrap()
        .to_string();
        sqlx::query!(
            "INSERT INTO users (id, username, password_hash) VALUES ($1, $2, $3)",
            self.id,
            self.username,
            password_hash
        )
        .execute(pool)
        .await
        .expect("failed to store test user");
    }
}

pub(crate) struct ConfirmationLinks {
    pub(crate) html: Url,
    pub(crate) text: Url,
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
