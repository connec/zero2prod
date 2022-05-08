use axum::{
    extract::rejection::{TypedHeaderRejection, TypedHeaderRejectionReason},
    headers::{authorization::Basic, Authorization},
    http::StatusCode,
    Extension, Json, TypedHeader,
};
use eyre::Context;
use uuid::Uuid;

use crate::{
    auth,
    domain::{self, SubscriberEmail},
    EmailClient, Error, Tx,
};

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
        Ok(TypedHeader(Authorization(credentials))) => auth::Credentials {
            username: credentials.username().to_string(),
            password: credentials.password().to_string(),
        },
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
    let user_id = auth::validate_credentials(&mut tx, credentials).await?;
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
