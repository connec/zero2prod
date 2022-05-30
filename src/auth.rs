use std::fmt;

use argon2::{Argon2, PasswordHash, PasswordVerifier};
use axum::{
    extract::{FromRequest, RequestParts},
    http::Request,
    response::IntoResponse,
    Extension,
};
use axum_extra::extract::{cookie::Cookie, SignedCookieJar};
use eyre::Context;
use redis::{AsyncCommands, FromRedisValue, ToRedisArgs};
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

#[derive(Clone, Copy)]
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

impl FromRedisValue for UserId {
    fn from_redis_value(v: &redis::Value) -> redis::RedisResult<Self> {
        if let redis::Value::Data(bytes) = v {
            if let Ok(uuid) = Uuid::parse_str(&String::from_utf8_lossy(bytes)) {
                Ok(Self(uuid))
            } else {
                Err((redis::ErrorKind::TypeError, "invalid UUID").into())
            }
        } else {
            Err((redis::ErrorKind::TypeError, "wrong data type for user ID").into())
        }
    }
}

impl ToRedisArgs for UserId {
    fn write_redis_args<W>(&self, out: &mut W)
    where
        W: ?Sized + redis::RedisWrite,
    {
        out.write_arg_fmt(self.0)
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

pub(crate) async fn session_middleware<B>(
    req: Request<B>,
    next: axum::middleware::Next<B>,
) -> Result<impl IntoResponse, crate::Error>
where
    B: Send,
{
    let mut req = RequestParts::new(req);

    let cookies: SignedCookieJar = req
        .extract()
        .await
        .context("missing cookie::Key in request extensions")?;

    let (cookies, session_id) = if let Some(session_id) = cookies.get("session_id") {
        (cookies, session_id.value().to_string())
    } else {
        let session_id = Uuid::new_v4().to_string();
        (
            cookies.add(Cookie::new("session_id", session_id.clone())),
            session_id,
        )
    };

    let Extension(session_client): Extension<redis::Client> = req
        .extract()
        .await
        .context("missing redis::Client in request extensions")?;

    let connection = session_client
        .get_async_connection()
        .await
        .context("could not connect to session store")?;

    req.extensions_mut().insert(Session {
        connection,
        session_id,
    });

    // unwrap is fine since we don't (and mustn't!) touch body above
    let req = req.try_into_request().unwrap();

    Ok((cookies, next.run(req).await))
}

// TODO: eager load (?) and lazy write
// TODO: use a typemap
pub(crate) struct Session {
    connection: redis::aio::Connection,
    session_id: String,
}

impl Session {
    pub(crate) async fn get<T: FromRedisValue>(
        &mut self,
        key: &str,
    ) -> redis::RedisResult<Option<T>> {
        self.connection.hget(&self.session_id, key).await
    }

    pub(crate) async fn insert<T: ToRedisArgs + Send + Sync>(
        &mut self,
        key: &str,
        value: T,
    ) -> redis::RedisResult<()> {
        self.connection.hset(&self.session_id, key, value).await
    }
}

impl<B: Send> FromRequest<B> for Session {
    type Rejection = crate::Error;

    fn from_request<'life0, 'async_trait>(
        req: &'life0 mut RequestParts<B>,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self, Self::Rejection>> + Send + 'async_trait>,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(async move {
            req.extensions_mut()
                .remove()
                .ok_or_else(|| eyre::Report::msg("no Session in request extensions").into())
        })
    }
}
