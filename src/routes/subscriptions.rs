use axum::{extract::Form, http::StatusCode, Extension};
use eyre::Context;
use reqwest::Url;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    domain::{self, NewSubscriber, SubscriberEmail, SubscriberName},
    email_client, AppBaseUrl, EmailClient, Error, Tx,
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

#[tracing::instrument(skip_all)]
pub(crate) async fn subscribe(
    mut tx: Tx,
    base_url: Extension<AppBaseUrl>,
    email_client: Extension<EmailClient>,
    Form(form): Form<Subscriber>,
) -> Result<StatusCode, Error> {
    let subscriber = form.try_into()?;

    let subscriber_id = insert_subscriber(&mut tx, &subscriber)
        .await
        .context("failed to persist subscriber")?;

    let token = insert_subscription_token(&mut tx, &subscriber_id)
        .await
        .context("failed to persist subscription token")?;

    send_confirmation_email(&base_url, &email_client, subscriber, &token)
        .await
        .context("failed to send confirmation email")?;

    Ok(StatusCode::OK)
}

async fn insert_subscriber(tx: &mut Tx, input: &NewSubscriber) -> Result<Uuid, sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at)
        VALUES ($1, $2, $3, $4)
        RETURNING id
        "#,
        Uuid::new_v4(),
        input.email.as_ref(),
        input.name.as_ref(),
        OffsetDateTime::now_utc(),
    )
    .fetch_one(tx)
    .await
    .map(|row| row.id)
}

#[tracing::instrument(skip_all)]
async fn insert_subscription_token(tx: &mut Tx, subscriber_id: &Uuid) -> Result<Uuid, sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO subscription_tokens (id, subscriber_id)
        VALUES ($1, $2)
        RETURNING id
        "#,
        Uuid::new_v4(),
        subscriber_id,
    )
    .fetch_one(tx)
    .await
    .map(|row| row.id)
}

#[tracing::instrument(skip_all)]
async fn send_confirmation_email(
    base_url: &Url,
    email_client: &EmailClient,
    subscriber: NewSubscriber,
    token: &Uuid,
) -> Result<(), email_client::Error> {
    let mut confirmation_link = base_url.join("/subscriptions/confirm").unwrap();
    confirmation_link
        .query_pairs_mut()
        .append_pair("token", &token.to_string());

    let html_body = format!(
        "Welcome to our newsletter!<br />\
        Click <a href=\"{}\">here</a> to confirm your subscription.",
        confirmation_link,
    );

    let text_body = format!(
        "Welcome to our newsletter!\n\
        Visit {} to confirm your subscription.",
        confirmation_link,
    );

    email_client
        .send_email(subscriber.email, "Welcome!", &html_body, &text_body)
        .await
}
