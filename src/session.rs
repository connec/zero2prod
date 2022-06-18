use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use axum::{
    body::Bytes,
    extract::{FromRequest, RequestParts},
    http::Request,
    middleware::Next,
    response::IntoResponse,
    Extension,
};
use axum_extra::extract::{cookie::Cookie, SignedCookieJar};
use futures::future::BoxFuture;
use redis::AsyncCommands as _;
use uuid::Uuid;

const SESSION_ID_COOKIE: &str = "session_id";

pub(crate) async fn middleware<B: Send>(
    req: Request<B>,
    next: Next<B>,
) -> Result<impl IntoResponse, Error> {
    let mut req = RequestParts::new(req);

    let slot = SessionSlot::default();
    req.extensions_mut().insert(slot.clone());

    let Extension(client): Extension<redis::Client> =
        req.extract().await.map_err(ErrorKind::MissingConfig)?;

    let cookies: SignedCookieJar = req.extract().await.map_err(ErrorKind::MissingConfig)?;

    // unwrap OK because we don't call take_body
    let res = next.run(req.try_into_request().unwrap()).await;

    let session = match slot.0.lock().unwrap().take() {
        Some(session) => session,
        None => return Ok((cookies, res)),
    };

    // unwrap OK because serializing to string can't fail and content is valid for serialization
    let session_bytes = serde_json::to_string(&session.content).unwrap();

    let mut connection = client
        .get_async_connection()
        .await
        .map_err(ErrorKind::Redis)?;

    let _: () = connection
        .set(&session.session_id.as_ref().unwrap(), session_bytes)
        .await
        .map_err(ErrorKind::Redis)?;

    Ok((
        cookies.add(Cookie::new(SESSION_ID_COOKIE, session.session_id.unwrap())),
        res,
    ))
}

#[derive(Clone, Default)]
struct SessionSlot(Arc<Mutex<Option<SessionFields>>>);

struct SessionFields {
    session_id: Option<String>,
    content: HashMap<String, serde_json::Value>,
}

pub(crate) struct Session {
    slot: SessionSlot,
    session_id: Option<String>,
    content: HashMap<String, serde_json::Value>,
    modified: bool,
}

impl Session {
    pub(crate) fn get<'a, T: serde::Deserialize<'a>>(&'a self, key: &str) -> Result<T, GetError> {
        let value = self.content.get(key).ok_or(GetError::NotFound)?;
        T::deserialize(value).map_err(GetError::De)
    }

    pub(crate) fn insert<T: serde::Serialize>(&mut self, key: &str, value: &T) {
        // unwrap because writing to a vec can't fail and we treat using a type that can't be
        // serialized to JSON as a programming error.
        let value = serde_json::to_value(value).unwrap();

        if let Some(existing) = self.content.get_mut(key) {
            *existing = value;
        } else {
            self.content.insert(key.to_owned(), value);
        }

        self.modified = true;
    }

    pub(crate) fn reset(&mut self) {
        self.session_id = Some(Uuid::new_v4().to_string());
        self.content.clear();
        self.modified = true;
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        if self.modified {
            *self.slot.0.lock().unwrap() = Some(SessionFields {
                session_id: std::mem::take(&mut self.session_id),
                content: std::mem::take(&mut self.content),
            });
        }
    }
}

impl<B: Send> FromRequest<B> for Session {
    type Rejection = Error;

    fn from_request<'req, 'ret>(
        req: &'req mut axum::extract::RequestParts<B>,
    ) -> BoxFuture<'ret, Result<Self, Self::Rejection>>
    where
        'req: 'ret,
        Self: 'ret,
    {
        Box::pin(async move {
            let Extension(slot): Extension<SessionSlot> =
                req.extract().await.map_err(ErrorKind::MissingConfig)?;

            let cookies: SignedCookieJar = req.extract().await.map_err(ErrorKind::MissingConfig)?;

            let session_id = match cookies.get(SESSION_ID_COOKIE) {
                Some(cookie) => cookie.value().to_owned(),
                None => {
                    return Ok(Session {
                        slot,
                        session_id: None,
                        content: HashMap::new(),
                        modified: false,
                    })
                }
            };

            let Extension(client): Extension<redis::Client> =
                req.extract().await.map_err(ErrorKind::MissingConfig)?;

            let mut connection = client
                .get_async_connection()
                .await
                .map_err(ErrorKind::Redis)?;

            let content_bytes: Bytes = connection
                .get(&session_id)
                .await
                .map_err(ErrorKind::Redis)?;

            let content =
                serde_json::from_slice(&content_bytes).map_err(ErrorKind::InvalidContent)?;

            Ok(Session {
                slot,
                session_id: Some(session_id),
                content,
                modified: false,
            })
        })
    }
}

#[derive(Debug, thiserror::Error)]
#[error("unable to load session")]
pub(crate) struct Error(#[from] ErrorKind);

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        crate::Error::Internal(self.into()).into_response()
    }
}

#[derive(Debug, thiserror::Error)]
enum ErrorKind {
    #[error(transparent)]
    MissingConfig(axum::extract::rejection::ExtensionRejection),

    #[error(transparent)]
    Redis(redis::RedisError),

    #[error(transparent)]
    InvalidContent(serde_json::Error),
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum GetError {
    #[error("field does not exist in session")]
    NotFound,

    #[error("field could not be deserialized")]
    De(serde_json::Error),
}
