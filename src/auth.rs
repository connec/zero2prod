use std::fmt;

use argon2::{Argon2, PasswordHash, PasswordVerifier};
use eyre::Context;
use uuid::Uuid;

use crate::Tx;

const AUTH_REALM: &str = "zero2prod";

// Used in the case of not-found users to ensure constant time authentication
const DEFAULT_PASSWORD_HASH: &str = "$argon2id$v=19$m=15000,t=2,p=1$gZiV/M1gPc22ElAH/Jh1Hw$CWOrkoo7oJBQ/iyh7uJ0LO2aLEfrHwTWllSAxT0zRno";

#[derive(Debug, thiserror::Error)]
pub(crate) enum Error {
    #[error("user is not authorized")]
    Unauthorized,

    #[error(transparent)]
    Unexpected(#[from] eyre::Report),
}

impl From<argon2::password_hash::Error> for Error {
    fn from(error: argon2::password_hash::Error) -> Self {
        Self::Unexpected(error.into())
    }
}

impl From<sqlx::Error> for Error {
    fn from(error: sqlx::Error) -> Self {
        Self::Unexpected(error.into())
    }
}

impl From<Error> for crate::Error {
    fn from(error: Error) -> Self {
        match error {
            Error::Unauthorized => Self::Unauthorized(AUTH_REALM.to_string()),
            Error::Unexpected(error) => {
                Self::Internal(error.wrap_err("an unexpected error occurred during authentication"))
            }
        }
    }
}

#[derive(Clone, Copy, serde::Deserialize, serde::Serialize)]
#[serde(transparent)]
pub(crate) struct UserId(Uuid);

impl std::ops::Deref for UserId {
    type Target = Uuid;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl fmt::Display for UserId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

pub(crate) struct Credentials {
    pub(crate) username: String,
    pub(crate) password: String,
}

#[tracing::instrument(skip_all)]
pub(crate) async fn validate_credentials(
    tx: &mut Tx,
    credentials: Credentials,
) -> Result<UserId, Error> {
    // This function should execute in the same amount of time, regardless of whether the user
    // exists or not.

    // Always allocate an owned default password, even if we won't use it
    let default_password_hash = DEFAULT_PASSWORD_HASH.to_string();

    let (user_id, password_hash) = match get_user(tx, &credentials.username).await? {
        Some((user_id, password_hash)) => (Some(user_id), password_hash),
        None => (None, default_password_hash),
    };

    // Always attempt to verify
    let valid = verify_password_hash(credentials, password_hash).await?;

    // Make the final decision based on whether the user exists and verification succeeded
    user_id
        .and_then(|user_id| valid.then(|| UserId(user_id)))
        .ok_or(Error::Unauthorized)
}

#[tracing::instrument(skip_all)]
async fn get_user(tx: &mut Tx, username: &str) -> Result<Option<(Uuid, String)>, sqlx::Error> {
    let user = sqlx::query!(
        "SELECT id, password_hash FROM users WHERE username = $1",
        username,
    )
    .fetch_optional(tx)
    .await?;

    Ok(user.map(|u| (u.id, u.password_hash)))
}

#[tracing::instrument(skip_all)]
async fn verify_password_hash(
    credentials: Credentials,
    password_hash: String,
) -> Result<bool, Error> {
    tokio::task::spawn_blocking(move || {
        let password_hash = PasswordHash::new(&password_hash)?;

        Ok(Argon2::default()
            .verify_password(credentials.password.as_bytes(), &password_hash)
            .is_ok())
    })
    .await
    .context("failed to spawn blocking task")?
}
