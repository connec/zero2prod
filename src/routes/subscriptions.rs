use axum::extract::Form;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{Error, Tx};

#[derive(serde::Deserialize)]
pub(crate) struct Subscriber {
    name: String,
    email: String,
}

#[tracing::instrument(skip(tx, form))]
pub(crate) async fn subscribe(mut tx: Tx, Form(form): Form<Subscriber>) -> Result<(), Error> {
    sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at)
        VALUES ($1, $2, $3, $4)
        "#,
        Uuid::new_v4(),
        form.email,
        form.name,
        OffsetDateTime::now_utc(),
    )
    .execute(&mut tx)
    .await?;
    Ok(())
}
