use std::{net::SocketAddr, time::Duration};

use axum::routing::{get, post};
use sqlx::postgres::PgPoolOptions;

use crate::{email_client::EmailClient, routes, telemetry, Config, Error};

fn routes() -> axum::Router {
    axum::Router::new()
        .route("/health", get(routes::health))
        .route("/subscriptions", post(routes::subscribe))
}

pub struct App {
    addr: SocketAddr,
    pool: sqlx::PgPool,
    service: axum::routing::IntoMakeService<axum::Router>,
}

pub type Server =
    axum::Server<hyper::server::conn::AddrIncoming, axum::routing::IntoMakeService<axum::Router>>;

impl App {
    pub fn new(config: &Config) -> Self {
        let pool = PgPoolOptions::new()
            .connect_timeout(Duration::from_secs(2))
            .connect_lazy_with(config.database_options());

        let email_client = EmailClient::new(
            config.email_base_url().clone(),
            config.email_sender().clone(),
            config.email_authorization_token().to_owned(),
            config.email_send_timeout(),
        );

        let service = routes()
            .layer(
                tower::ServiceBuilder::new()
                    .layer(telemetry::id_layer())
                    .layer(telemetry::trace_layer())
                    .layer(axum_sqlx_tx::Layer::new_with_error::<Error>(pool.clone()))
                    .layer(axum::Extension(email_client)),
            )
            .into_make_service();

        Self {
            addr: config.addr(),
            pool,
            service,
        }
    }

    pub fn pool(&self) -> &sqlx::PgPool {
        &self.pool
    }

    pub async fn serve(self) -> Result<Server, sqlx::migrate::MigrateError> {
        self.migrate().await?;
        Ok(axum::Server::bind(&self.addr).serve(self.service))
    }

    #[tracing::instrument(skip(self))]
    async fn migrate(&self) -> Result<(), sqlx::migrate::MigrateError> {
        sqlx::migrate!("./migrations").run(&self.pool).await
    }
}
