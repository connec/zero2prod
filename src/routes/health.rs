use hyper::StatusCode;

pub(crate) async fn health() -> StatusCode {
    StatusCode::NO_CONTENT
}
