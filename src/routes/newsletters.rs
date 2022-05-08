use argon2::{Argon2, PasswordHash, PasswordVerifier};
use axum::{
    extract::rejection::{TypedHeaderRejection, TypedHeaderRejectionReason},
    headers::{authorization::Basic, Authorization},
    http::StatusCode,
    Extension, Json, TypedHeader,
};
use eyre::Context;
use uuid::Uuid;

use crate::{
    domain::{self, SubscriberEmail},
    EmailClient, Error, Tx,
};

const AUTH_REALM: &str = "publish";

// Used in the case of not-found users to ensure constant time authentication
const DEFAULT_PASSWORD_HASH: &str = "$argon2id$v=19$m=15000,t=2,p=1$gZiV/M1gPc22ElAH/Jh1Hw$CWOrkoo7oJBQ/iyh7uJ0LO2aLEfrHwTWllSAxT0zRno";

#[derive(serde::Deserialize)]
pub(crate) struct Newsletter {
    title: String,
    content: Content,
}

#[derive(serde::Deserialize)]
struct Content {
    html: String,
    text: String,
}

struct ConfirmedSubscriber {
    id: Uuid,
    email: SubscriberEmail,
}

#[tracing::instrument(skip_all, fields(user_id=tracing::field::Empty))]
pub(crate) async fn publish_newsletter(
    mut tx: Tx,
    email_client: Extension<EmailClient>,
    authorization: Result<TypedHeader<Authorization<Basic>>, TypedHeaderRejection>,
    Json(newsletter): Json<Newsletter>,
) -> Result<StatusCode, Error> {
    // TODO: move into an extractor once the implementation is stable
    let credentials = match authorization {
        Ok(TypedHeader(Authorization(credentials))) => credentials,
        Err(rejection) if matches!(rejection.reason(), TypedHeaderRejectionReason::Missing) => {
            return Err(Error::Unauthorized("publish".to_string()))
        }
        Err(error) => {
            return Err(Error::Validation(format!(
                "invalid authorization header: {}",
                error
            )))
        }
    };
    let user_id =
        validate_credentials(&mut tx, credentials)
            .await
            .map_err(|error| match error {
                Error::Internal(error) => {
                    Error::Internal(error.wrap_err("failed to validate credentials"))
                }
                _ => error,
            })?;
    tracing::Span::current().record("user_id", &tracing::field::display(user_id));

    let subscribers = get_confirmed_subscribers(&mut tx)
        .await
        .context("failed to retrieve confirmed subscribers")?;

    for subscriber in subscribers {
        let subscriber = match subscriber {
            Ok(subscriber) => subscriber,
            Err((subscriber_id, error)) => {
                tracing::warn!(%subscriber_id, %error, "skipping confirmed subscriber with invalid email");
                continue;
            }
        };

        email_client
            .send_email(
                subscriber.email,
                &newsletter.title,
                &newsletter.content.html,
                &newsletter.content.text,
            )
            .await
            .with_context(|| format!("failed to send a newsletter email to {}", subscriber.id))?;
    }

    Ok(StatusCode::OK)
}

#[tracing::instrument(skip_all)]
async fn validate_credentials(tx: &mut Tx, credentials: Basic) -> Result<Uuid, Error> {
    // This function should execute in the same amount of time, regardless of whether the user
    // exists or not.

    // Always allocate an owned default password, even if we won't use it
    let default_password_hash = DEFAULT_PASSWORD_HASH.to_string();

    let (user_id, password_hash) = match get_user(tx, credentials.username()).await? {
        Some((user_id, password_hash)) => (Some(user_id), password_hash),
        None => (None, default_password_hash),
    };

    // Always attempt to verify
    let valid = verify_password_hash(credentials, password_hash).await?;

    // Make the final decision based on whether the user exists and verification succeeded
    user_id
        .and_then(|user_id| valid.then(|| user_id))
        .ok_or_else(|| Error::Unauthorized(AUTH_REALM.to_string()))
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
async fn verify_password_hash(credentials: Basic, password_hash: String) -> Result<bool, Error> {
    tokio::task::spawn_blocking(move || {
        let password_hash =
            PasswordHash::new(&password_hash).map_err(|error| Error::Internal(error.into()))?;

        Ok(Argon2::default()
            .verify_password(credentials.password().as_bytes(), &password_hash)
            .is_ok())
    })
    .await
    .context("failed to spawn blocking task")?
}

type ConfirmedSubscriberResult = Result<ConfirmedSubscriber, (Uuid, domain::Error)>;

#[tracing::instrument(skip_all)]
async fn get_confirmed_subscribers(
    tx: &mut Tx,
) -> Result<Vec<ConfirmedSubscriberResult>, sqlx::Error> {
    let rows = sqlx::query!("SELECT id, email FROM subscriptions WHERE status = 'confirmed'")
        .fetch_all(tx)
        .await?;

    Ok(rows
        .into_iter()
        .map(|row| {
            let email = SubscriberEmail::parse(row.email).map_err(|error| (row.id, error))?;
            Ok(ConfirmedSubscriber { id: row.id, email })
        })
        .collect())
}
