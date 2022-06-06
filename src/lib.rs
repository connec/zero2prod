mod app;
mod auth;
mod config;
mod domain;
mod email_client;
mod error;
mod routes;
mod session;
pub mod telemetry;

pub use self::{
    app::{App, AppBaseUrl, Server},
    config::Config,
    email_client::EmailClient,
};

pub(crate) use self::error::Error;

pub(crate) type Tx = axum_sqlx_tx::Tx<sqlx::Postgres, Error>;
