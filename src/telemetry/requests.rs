use std::{convert::Infallible, fmt, marker::PhantomData, sync::Arc, task, time::Duration};

use axum::{body::Body, http::Request, response::Response};
use eyre::Report;
use futures::future::BoxFuture;
use tower::{Layer, Service};
use tower_http::{
    classify::{ClassifiedResponse, ClassifyResponse, NeverClassifyEos, SharedClassifier},
    trace::{DefaultOnBodyChunk, DefaultOnEos, TraceLayer},
};
use tracing::Span;
use uuid::Uuid;

#[derive(Clone)]
pub(crate) struct Id(Uuid);

impl Default for Id {
    fn default() -> Self {
        Self(Uuid::new_v4())
    }
}

impl fmt::Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub(crate) struct IdLayer<S>(PhantomData<S>);

impl<S> Layer<S> for IdLayer<S> {
    type Service = IdMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        IdMiddleware { inner }
    }
}

#[derive(Clone)]
pub(crate) struct IdMiddleware<S> {
    inner: S,
}

impl<S, ReqBody, ResBody> Service<Request<ReqBody>> for IdMiddleware<S>
where
    S: Service<Request<ReqBody>, Response = Response<ResBody>, Error = Infallible>,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut task::Context<'_>) -> task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request<ReqBody>) -> Self::Future {
        let id = Id::default();

        req.extensions_mut().insert(id.clone());

        let res = self.inner.call(req);

        Box::pin(async move {
            let mut res = res.await?;
            res.headers_mut()
                // we unwrap the parsed header value because we know UUIDs will be valid
                .insert("request-id", id.to_string().parse().unwrap());
            Ok(res)
        })
    }
}

pub(crate) fn id_layer<S>() -> IdLayer<S> {
    IdLayer(PhantomData)
}

pub(crate) fn trace_layer() -> TraceLayer<
    SharedClassifier<Classifier>,
    impl (FnMut(&Request<Body>) -> Span) + Clone,
    impl FnMut(&Request<Body>, &Span) + Clone,
    impl FnOnce(&Response, Duration, &Span) + Clone,
    DefaultOnBodyChunk,
    DefaultOnEos,
    impl FnMut(Arc<Report>, Duration, &Span) + Clone,
> {
    TraceLayer::new(SharedClassifier::new(Classifier::default()))
        .make_span_with(|request: &Request<Body>| {
            let id = request
                .extensions()
                .get()
                .cloned()
                .unwrap_or(Id(Uuid::nil()));
            tracing::debug_span!(
                "request",
                %id,
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
        .on_failure(|error: Arc<Report>, latency: Duration, _span: &Span| {
            tracing::error!(
                error = ?error.as_ref(),
                latency = %format_args!("{}ms", latency.as_millis()),
                "error processing request",
            );
        })
}

#[derive(Clone, Default)]
pub(crate) struct Classifier;

impl ClassifyResponse for Classifier {
    type FailureClass = Arc<Report>;
    type ClassifyEos = NeverClassifyEos<Self::FailureClass>;

    fn classify_response<B>(
        self,
        response: &Response<B>,
    ) -> ClassifiedResponse<Self::FailureClass, Self::ClassifyEos> {
        let error: Option<&Arc<Report>> = response.extensions().get();
        if let Some(error) = error {
            ClassifiedResponse::Ready(Err(error.clone()))
        } else if response.status().is_server_error() {
            ClassifiedResponse::Ready(Err(Arc::new(Report::msg("error not recorded"))))
        } else {
            ClassifiedResponse::Ready(Ok(()))
        }
    }

    fn classify_error<E>(self, error: &E) -> Self::FailureClass
    where
        E: std::fmt::Display + 'static,
    {
        Arc::new(Report::msg(error.to_string()))
    }
}
