use axum::response::Html;

#[tracing::instrument(skip_all)]
pub(crate) async fn home() -> Html<&'static str> {
    Html(include_str!("index.html"))
}
