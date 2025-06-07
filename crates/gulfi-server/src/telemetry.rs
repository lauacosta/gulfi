use color_eyre::owo_colors::OwoColorize;
use std::fmt;
use tracing::Subscriber;
use tracing_error::ErrorLayer;
use tracing_log::LogTracer;
use tracing_subscriber::{EnvFilter, Registry, fmt::Layer, layer::SubscriberExt};

pub fn get_subscriber(env_filter: String) -> impl Subscriber + Send + Sync {
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(env_filter));

    Registry::default()
        .with(env_filter)
        .with(
            Layer::new()
                .compact()
                .with_ansi(true)
                .with_timer(GulfiTimer::new())
                .with_span_events(tracing_subscriber::fmt::format::FmtSpan::FULL),
        )
        .with(ErrorLayer::default())
}

pub fn init_subscriber(subscriber: impl Subscriber + Send + Sync) {
    LogTracer::init().expect("Failed to set logger");
    tracing::subscriber::set_global_default(subscriber).expect("Failed to set subscriber");
}

struct GulfiTimer;

impl GulfiTimer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for GulfiTimer {
    fn default() -> Self {
        Self::new()
    }
}

impl tracing_subscriber::fmt::time::FormatTime for GulfiTimer {
    fn format_time(&self, w: &mut tracing_subscriber::fmt::format::Writer<'_>) -> fmt::Result {
        let datetime = chrono::Local::now().format("%H:%M:%S");
        let str = format!("{}", datetime.bright_blue());

        write!(w, "{str}")?;
        Ok(())
    }
}
