use std::{net::SocketAddr, time::Duration};

use axum::routing::{get, post};
use reqwest::Url;
use sqlx::postgres::PgPoolOptions;
use tracing::warn;

use crate::{email_client::EmailClient, routes, telemetry, Config, Error};

fn routes() -> axum::Router {
    axum::Router::new()
        .route("/health", get(routes::health))
        .route("/subscriptions", post(routes::subscribe))
        .route("/subscriptions/confirm", get(routes::confirm))
}

pub struct App {
    addr: SocketAddr,
    pool: sqlx::PgPool,
    ignore_missing_migrations: bool,
    service: axum::routing::IntoMakeService<axum::Router>,
}

pub type Server =
    axum::Server<hyper::server::conn::AddrIncoming, axum::routing::IntoMakeService<axum::Router>>;

#[derive(Clone)]
pub struct AppBaseUrl(Url);

impl std::ops::Deref for AppBaseUrl {
    type Target = Url;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl App {
    pub fn new(config: Config) -> Self {
        let pool = PgPoolOptions::new()
            .connect_timeout(Duration::from_secs(2))
            .connect_lazy_with(config.database_options());

        let email_client = EmailClient::new(
            config.email_base_url,
            config.email_sender,
            config.email_authorization_token,
            config.email_send_timeout,
        );

        let service = routes()
            .layer(
                tower::ServiceBuilder::new()
                    .layer(telemetry::id_layer())
                    .layer(telemetry::trace_layer())
                    .layer(axum_sqlx_tx::Layer::new_with_error::<Error>(pool.clone()))
                    .layer(axum::Extension(AppBaseUrl(config.base_url)))
                    .layer(axum::Extension(email_client)),
            )
            .into_make_service();

        Self {
            addr: config.address,
            pool,
            ignore_missing_migrations: config.ignore_missing_migrations,
            service,
        }
    }

    pub fn pool(&self) -> &sqlx::PgPool {
        &self.pool
    }

    pub async fn serve(self) -> Result<Server, sqlx::migrate::MigrateError> {
        self.migrate().await.or_else(|error| match error {
            sqlx::migrate::MigrateError::VersionMissing(_) if self.ignore_missing_migrations => {
                warn!(
                    ?error,
                    "database state is ahead of that known by the app ??? \
                    in a rollback scenario this is expected, but otherwise something may be wrong"
                );
                Ok(())
            }
            _ => Err(error),
        })?;
        Ok(axum::Server::bind(&self.addr).serve(self.service))
    }

    #[tracing::instrument(skip(self))]
    async fn migrate(&self) -> Result<(), sqlx::migrate::MigrateError> {
        sqlx::migrate!("./migrations").run(&self.pool).await
    }
}
