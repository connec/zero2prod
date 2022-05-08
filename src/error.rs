use std::{fmt, sync::Arc};

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

#[derive(Debug)]
pub(crate) enum Error {
    Validation(String),
    Internal(eyre::Report),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Validation(error) => write!(f, "{}", error),
            Error::Internal(error) => write!(f, "{}", error),
        }
    }
}

impl From<sqlx::Error> for Error {
    fn from(error: sqlx::Error) -> Self {
        Self::Internal(error.into())
    }
}

impl From<axum_sqlx_tx::Error> for Error {
    fn from(error: axum_sqlx_tx::Error) -> Self {
        Self::Internal(error.into())
    }
}

impl From<eyre::Report> for Error {
    fn from(error: eyre::Report) -> Self {
        Self::Internal(error)
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        match self {
            Self::Validation(error) => (StatusCode::UNPROCESSABLE_ENTITY, error).into_response(),
            Self::Internal(error) => {
                let mut response = StatusCode::INTERNAL_SERVER_ERROR.into_response();
                response.extensions_mut().insert(Arc::new(error));
                response
            }
        }
    }
}
