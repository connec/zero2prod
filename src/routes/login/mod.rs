use axum::{
    extract::Form,
    response::{Html, Redirect},
};
use axum_extra::extract::{cookie::Cookie, SignedCookieJar};
use eyre::Context;

use crate::{auth, session::Session, Error, Tx};

#[tracing::instrument(skip_all)]
pub(crate) async fn login_form(cookies: SignedCookieJar) -> (SignedCookieJar, Html<String>) {
    let error_html = if let Some(cookie) = cookies.get("_flash") {
        format!("<p><i>{}</i></p>", cookie.value())
    } else {
        "".to_string()
    };
    (
        cookies.remove(Cookie::named("_flash")),
        Html(format!(include_str!("index.html"), error_html = error_html)),
    )
}

#[derive(serde::Deserialize)]
pub(crate) struct Credentials {
    username: String,
    password: String,
}

impl From<Credentials> for auth::Credentials {
    fn from(credentials: Credentials) -> Self {
        Self {
            username: credentials.username,
            password: credentials.password,
        }
    }
}

#[tracing::instrument(skip_all, fields(user_id = tracing::field::Empty))]
pub(crate) async fn login(
    mut tx: Tx,
    cookies: SignedCookieJar,
    mut session: Session,
    Form(credentials): Form<Credentials>,
) -> Result<(SignedCookieJar, Redirect), Error> {
    match auth::validate_credentials(&mut tx, credentials.into()).await {
        Ok(user_id) => {
            session
                .insert("user_id", user_id)
                .await
                .context("failed to store user_id in session")?;
            tracing::Span::current().record("user_id", &tracing::field::display(&user_id));
            Ok((cookies, Redirect::to("/admin/dashboard")))
        }
        Err(auth::Error::Unauthorized) => Ok((
            cookies.add(
                Cookie::build("_flash", "Authentication failed")
                    .http_only(true)
                    .secure(true)
                    .finish(),
            ),
            Redirect::to("/login"),
        )),
        Err(error) => Err(error.into()),
    }
}
