use std::time::Duration;

use axum::{body::Body, http::Request, response::Response};
use tower_http::{
    classify::{
        ClassifiedResponse, ClassifyResponse, NeverClassifyEos, ServerErrorsAsFailures,
        SharedClassifier,
    },
    trace::{DefaultOnBodyChunk, DefaultOnEos, TraceLayer},
};
use tracing::Span;

use crate::Error;

pub(crate) fn layer() -> TraceLayer<
    SharedClassifier<Classifier>,
    impl (FnMut(&Request<Body>) -> Span) + Clone,
    impl FnMut(&Request<Body>, &Span) + Clone,
    impl FnOnce(&Response, Duration, &Span) + Clone,
    DefaultOnBodyChunk,
    DefaultOnEos,
    impl FnMut(Error, Duration, &Span) + Clone,
> {
    TraceLayer::new(SharedClassifier::new(Classifier::default()))
        .make_span_with(|request: &Request<Body>| {
            tracing::debug_span!(
                "request",
                method = %request.method(),
                uri = %request.uri(),
                version = ?request.version(),
            )
        })
        .on_request(|_request: &Request<Body>, _span: &Span| {
            tracing::debug!("started processing request")
        })
        .on_response(|response: &Response, latency: Duration, _span: &Span| {
            tracing::debug!(
                status = %response.status().as_u16(),
                latency = %format_args!("{}ms", latency.as_millis()),
                "finished processing request",
            )
        })
        .on_failure(|error: Error, latency: Duration, _span: &Span| {
            tracing::error!(
                %error,
                latency = %format_args!("{}ms", latency.as_millis()),
                "error processing request",
            );
        })
}

#[derive(Clone, Default)]
pub(crate) struct Classifier {
    fallback: ServerErrorsAsFailures,
}

impl ClassifyResponse for Classifier {
    type FailureClass = Error;
    type ClassifyEos = NeverClassifyEos<Self::FailureClass>;

    fn classify_response<B>(
        self,
        response: &Response<B>,
    ) -> ClassifiedResponse<Self::FailureClass, Self::ClassifyEos> {
        let error: Option<&Error> = response.extensions().get();
        if let Some(error) = error {
            return ClassifiedResponse::Ready(Err(error.clone()));
        }

        match self.fallback.classify_response(response) {
            ClassifiedResponse::Ready(res) => {
                ClassifiedResponse::Ready(res.map_err(|error| Error(error.to_string())))
            }
            // `NeverClassifyEos` values cannot exist (it uses `Infallible` internally)
            ClassifiedResponse::RequiresEos(_) => unreachable!(),
        }
    }

    fn classify_error<E>(self, error: &E) -> Self::FailureClass
    where
        E: std::fmt::Display + 'static,
    {
        Error(error.to_string())
    }
}
