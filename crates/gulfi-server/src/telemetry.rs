use color_eyre::owo_colors::OwoColorize;
use opentelemetry::{global, trace::TracerProvider as _};
use opentelemetry_otlp::{SpanExporterBuilder, WithExportConfig, WithHttpConfig};
use opentelemetry_sdk::{
    Resource,
    trace::{BatchConfigBuilder, BatchSpanProcessor, SdkTracerProvider},
};
use secrecy::ExposeSecret;
use std::{collections::HashMap, fmt, time::Duration};
use tracing::Subscriber;
use tracing_error::ErrorLayer;
use tracing_log::LogTracer;
use tracing_subscriber::{
    EnvFilter, Registry,
    fmt::{Layer, format::FmtSpan},
    layer::SubscriberExt,
};

use crate::configuration::Settings;

pub fn get_subscriber(
    configuration: &Settings,
    env_filter: String,
) -> impl Subscriber + Send + Sync {
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(env_filter));

    let mut headers = HashMap::new();
    headers.insert(
        "x-honeycomb-team".to_owned(),
        configuration
            .tracer_provider
            .api_key
            .expose_secret()
            .to_string(),
    );

    let otlp_exporter = SpanExporterBuilder::default()
        .with_http()
        .with_protocol(configuration.tracer_provider.protocol)
        .with_headers(headers)
        .with_endpoint(configuration.tracer_provider.endpoint.clone())
        .build()
        .expect("OTLP Exporter should build");

    // TODO: Add options in the config
    let batch_processor = BatchSpanProcessor::builder(otlp_exporter)
        .with_batch_config(
            BatchConfigBuilder::default()
                .with_max_export_batch_size(512)
                // .with_max_export_timeout(Duration::from_secs(30))
                .with_scheduled_delay(Duration::from_millis(500))
                .build(),
        )
        .build();

    let provider = SdkTracerProvider::builder()
        .with_span_processor(batch_processor)
        .with_resource(
            Resource::builder()
                .with_service_name(configuration.tracer_provider.service_name.clone())
                .build(),
        )
        .build();

    global::set_tracer_provider(provider.clone());

    let tracer = provider.tracer("gulfi-server");
    let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);

    Registry::default()
        .with(env_filter)
        .with(telemetry)
        .with(
            Layer::new()
                .compact()
                .with_ansi(true)
                .with_timer(GulfiTimer::new())
                .with_span_events(FmtSpan::CLOSE),
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
