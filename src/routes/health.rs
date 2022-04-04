use hyper::StatusCode;

#[tracing::instrument]
pub(crate) async fn health() -> StatusCode {
    StatusCode::NO_CONTENT
}
