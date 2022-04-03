use axum::extract::Form;
use chrono::Utc;
use hyper::StatusCode;
use uuid::Uuid;

use crate::Tx;

#[derive(serde::Deserialize)]
pub(crate) struct Subscriber {
    name: String,
    email: String,
}

pub(crate) async fn subscribe(mut tx: Tx, Form(form): Form<Subscriber>) -> StatusCode {
    match sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at)
        VALUES ($1, $2, $3, $4)
        "#,
        Uuid::new_v4(),
        form.email,
        form.name,
        Utc::now(),
    )
    .execute(&mut tx)
    .await
    {
        Ok(_) => StatusCode::OK,
        Err(error) => {
            eprintln!("Failed to execute query: {}", error);
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}
