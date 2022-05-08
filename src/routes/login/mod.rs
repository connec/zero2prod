use axum::{
    extract::{Form, Query},
    response::{Html, Redirect},
    Extension,
};

use crate::{auth, AppBaseUrl, Error, Tx};

#[derive(serde::Deserialize)]
pub(crate) struct LoginFormParams {
    error: Option<String>,
}

#[tracing::instrument(skip_all)]
pub(crate) async fn login_form(params: Option<Query<LoginFormParams>>) -> Html<String> {
    let error_html = if params.and_then(|params| params.0.error).is_some() {
        "<p><i>Invalid username or password</i></p>"
    } else {
        ""
    };
    Html(format!(include_str!("index.html"), error_html = error_html))
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
    Extension(base_url): Extension<AppBaseUrl>,
    Form(credentials): Form<Credentials>,
) -> Result<Redirect, Error> {
    let user_id = auth::validate_credentials(&mut tx, credentials.into())
        .await
        .map(Some)
        .or_else(|error| match error {
            auth::Error::Unauthorized => Ok(None),
            _ => Err(error),
        })?;

    if let Some(user_id) = user_id {
        tracing::Span::current().record("user_id", &tracing::field::display(&user_id));
        Ok(Redirect::to("/"))
    } else {
        let mut url = base_url.join("/login").expect("invalid base URL");
        url.query_pairs_mut().append_key_only("error");

        Ok(Redirect::to(url.as_str()))
    }
}
