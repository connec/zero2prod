use axum::{
    extract::{FromRequest, RequestParts},
    http::Request,
    response::IntoResponse,
    Extension,
};
use axum_extra::extract::{cookie::Cookie, SignedCookieJar};
use eyre::Context as _;
use redis::AsyncCommands as _;
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub(crate) struct Error(ErrorRepr);

#[derive(Debug, thiserror::Error)]
enum ErrorRepr {
    #[error(transparent)]
    Redis(redis::RedisError),

    #[error(transparent)]
    Serialization(serde_json::Error),
}

impl From<redis::RedisError> for Error {
    fn from(error: redis::RedisError) -> Self {
        Self(ErrorRepr::Redis(error))
    }
}

impl From<serde_json::Error> for Error {
    fn from(error: serde_json::Error) -> Self {
        Self(ErrorRepr::Serialization(error))
    }
}

pub(crate) async fn middleware<B>(
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
    pub(crate) async fn get<T: serde::de::DeserializeOwned>(
        &mut self,
        key: &str,
    ) -> Result<Option<T>, Error> {
        let stored: String = self.connection.hget(&self.session_id, key).await?;
        let value = serde_json::from_str(&stored)?;
        Ok(value)
    }

    pub(crate) async fn insert<T: serde::Serialize + Send + Sync>(
        &mut self,
        key: &str,
        value: T,
    ) -> Result<(), Error> {
        let stored = serde_json::to_string(&value)?;
        self.connection.hset(&self.session_id, key, stored).await?;
        Ok(())
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
