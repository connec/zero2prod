use axum::extract::Form;
use hyper::StatusCode;

#[derive(serde::Deserialize)]
pub(crate) struct Subscriber {
    name: String,
    email: String,
}

pub(crate) async fn subscribe(Form(form): Form<Subscriber>) -> StatusCode {
    StatusCode::OK
}
