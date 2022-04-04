mod requests;

use std::env;

use tracing::Level;
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_subscriber::{
    filter::Targets, fmt::MakeWriter, layer::SubscriberExt as _, util::SubscriberInitExt as _,
};

pub(crate) use self::requests::{id_layer, trace_layer};

pub fn init<Sink>(name: impl ToString, sink: Sink)
where
    Sink: for<'a> MakeWriter<'a> + Send + Sync + 'static,
{
    let name = name.to_string();

    let filter = if let Ok(filter) = env::var("RUST_LOG") {
        filter.parse().expect("invalid configuration for RUST_LOG")
    } else {
        Targets::new()
    }
    .with_target(env!("CARGO_PKG_NAME"), Level::DEBUG)
    .with_target(&name, Level::DEBUG);

    tracing_subscriber::registry()
        .with(filter)
        .with(JsonStorageLayer)
        .with(BunyanFormattingLayer::new(name, sink))
        .init();
}
