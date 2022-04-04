mod config;
mod request_tracing;
mod routes;
mod server;

use std::fmt;

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

pub use self::{
    config::Config,
    server::{bind, Server, ServerResult},
};

pub(crate) type Tx = axum_sqlx_tx::Tx<sqlx::Postgres, Error>;

#[derive(Clone, Debug)]
pub(crate) struct Error(String);

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<sqlx::Error> for Error {
    fn from(error: sqlx::Error) -> Self {
        Self(error.to_string())
    }
}

impl From<axum_sqlx_tx::Error> for Error {
    fn from(error: axum_sqlx_tx::Error) -> Self {
        Self(error.to_string())
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let mut response = StatusCode::INTERNAL_SERVER_ERROR.into_response();
        response.extensions_mut().insert(self);
        response
    }
}
