use eyre::Context;
use uuid::Uuid;

use crate::{
    auth::{Session, UserId},
    Error, Tx,
};

pub(crate) async fn admin_dashboard(mut tx: Tx, mut session: Session) -> Result<String, Error> {
    let username = if let Some(user_id) = session
        .get::<UserId>("user_id")
        .await
        .context("failed to retrieve user_id from session")?
    {
        get_username(&mut tx, *user_id).await?
    } else {
        todo!()
    };
    Ok(format!(include_str!("dashboard.html"), username = username))
}

#[tracing::instrument(skip(tx))]
async fn get_username(tx: &mut Tx, user_id: Uuid) -> Result<String, sqlx::Error> {
    sqlx::query!("SELECT username FROM users WHERE id = $1", user_id)
        .fetch_one(tx)
        .await
        .map(|row| row.username)
}
