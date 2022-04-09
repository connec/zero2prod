use axum::{extract::Form, http::StatusCode};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    domain::{self, NewSubscriber, SubscriberEmail, SubscriberName},
    Error, Tx,
};

#[derive(serde::Deserialize)]
pub(crate) struct Subscriber {
    name: String,
    email: String,
}

impl TryFrom<Subscriber> for NewSubscriber {
    type Error = domain::Error;

    fn try_from(value: Subscriber) -> Result<Self, Self::Error> {
        Ok(NewSubscriber {
            email: SubscriberEmail::parse(value.email)?,
            name: SubscriberName::parse(value.name)?,
        })
    }
}

#[tracing::instrument(skip(tx, form))]
pub(crate) async fn subscribe(
    mut tx: Tx,
    Form(form): Form<Subscriber>,
) -> Result<StatusCode, Error> {
    let input = form.try_into()?;

    insert_subscriber(&mut tx, input).await?;

    Ok(StatusCode::OK)
}

async fn insert_subscriber(tx: &mut Tx, input: NewSubscriber) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at)
        VALUES ($1, $2, $3, $4)
        "#,
        Uuid::new_v4(),
        input.email.as_ref(),
        input.name.as_ref(),
        OffsetDateTime::now_utc(),
    )
    .execute(tx)
    .await?;

    Ok(())
}
