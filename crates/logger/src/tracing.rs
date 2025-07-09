use std::env::var;

use tracing::{level_filters::LevelFilter, warn};
use tracing_subscriber::{Layer, filter::EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

pub fn init() {
    initialize_tracing(LevelFilter::INFO);
}

/// Initialize tracing subscriber with default configuration.
fn initialize_tracing(level: LevelFilter) {
    let env_filter = EnvFilter::builder().with_default_directive(level.into()).from_env_lossy();

    let log_format = var("RUST_LOG_FORMAT")
        .inspect_err(|error| {
            warn!("Failed to read RUST_LOG_FORMAT, falling back to default: {error}")
        })
        .unwrap_or_default();

    let log_layer = match log_format.as_str() {
        "json" => tracing_subscriber::fmt::layer().json().with_filter(env_filter).boxed(),
        _ => tracing_subscriber::fmt::layer()
            .compact()
            .without_time()
            .with_filter(env_filter)
            .boxed(),
    };

    tracing_subscriber::registry().with(log_layer).init();
}
