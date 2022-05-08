use std::{fmt, sync::Arc};

use axum::{
    http::{header::WWW_AUTHENTICATE, StatusCode},
    response::{IntoResponse, Response},
};

#[derive(Debug)]
pub(crate) enum Error {
    Unauthorized(String),
    Validation(String),
    Internal(eyre::Report),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Unauthorized(realm) => write!(f, "unauthorized for realm {}", realm),
            Self::Validation(error) => write!(f, "{}", error),
            Self::Internal(error) => write!(f, "{}", error),
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
            Self::Unauthorized(realm) => (
                StatusCode::UNAUTHORIZED,
                [(WWW_AUTHENTICATE, format!("Basic realm=\"{}\"", realm))],
            )
                .into_response(),
            Self::Validation(error) => (StatusCode::UNPROCESSABLE_ENTITY, error).into_response(),
            Self::Internal(error) => {
                let mut response = StatusCode::INTERNAL_SERVER_ERROR.into_response();
                response.extensions_mut().insert(Arc::new(error));
                response
            }
        }
    }
}
