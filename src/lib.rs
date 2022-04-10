mod app;
mod config;
mod domain;
mod email_client;
mod routes;
pub mod telemetry;

use std::fmt;

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

pub use self::{
    app::{App, Server},
    config::Config,
    email_client::EmailClient,
};

pub(crate) type Tx = axum_sqlx_tx::Tx<sqlx::Postgres, Error>;

#[derive(Clone, Debug)]
pub(crate) enum Error {
    Validation(String),
    Internal(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Error::Validation(error) => error,
                Error::Internal(error) => error,
            }
        )
    }
}

impl From<sqlx::Error> for Error {
    fn from(error: sqlx::Error) -> Self {
        Self::Internal(error.to_string())
    }
}

impl From<axum_sqlx_tx::Error> for Error {
    fn from(error: axum_sqlx_tx::Error) -> Self {
        Self::Internal(error.to_string())
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        match self {
            Self::Validation(error) => (StatusCode::UNPROCESSABLE_ENTITY, error).into_response(),
            error @ Self::Internal(_) => {
                let mut response = StatusCode::INTERNAL_SERVER_ERROR.into_response();
                response.extensions_mut().insert(error);
                response
            }
        }
    }
}
