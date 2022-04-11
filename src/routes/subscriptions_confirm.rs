use axum::{extract::Query, http::StatusCode};
use uuid::Uuid;

use crate::{domain::SubscriberStatus, Error, Tx};

#[tracing::instrument(skip_all)]
pub(crate) async fn confirm(mut tx: Tx, params: Query<Params>) -> Result<StatusCode, Error> {
    let subscriber_id = match get_subscriber_id(&mut tx, &params.token).await? {
        None => return Ok(StatusCode::UNAUTHORIZED),
        Some(id) => id,
    };

    confirm_subscription(&mut tx, &subscriber_id).await?;

    Ok(StatusCode::OK)
}

#[derive(serde::Deserialize)]
pub(crate) struct Params {
    token: Uuid,
}

#[tracing::instrument(skip_all)]
async fn get_subscriber_id(
    tx: &mut Tx,
    subscription_token_id: &Uuid,
) -> Result<Option<Uuid>, sqlx::Error> {
    let row = sqlx::query!(
        r#"SELECT subscriber_id FROM subscription_tokens WHERE id = $1"#,
        subscription_token_id,
    )
    .fetch_optional(tx)
    .await?;

    Ok(row.map(|row| row.subscriber_id))
}

#[tracing::instrument(skip_all)]
async fn confirm_subscription(tx: &mut Tx, subscriber_id: &Uuid) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        UPDATE subscriptions
        SET status = $1
        WHERE subscriptions.id = $2
        "#,
        SubscriberStatus::Confirmed.as_str(),
        subscriber_id,
    )
    .execute(tx)
    .await?;

    Ok(())
}
