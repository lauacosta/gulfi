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
use tracing_subscriber::{EnvFilter, Layer, Registry, fmt::format::FmtSpan, layer::SubscriberExt};

use crate::configuration::Settings;

pub fn get_subscriber(
    configuration: &Settings,
    env_filter: String,
) -> impl Subscriber + Send + Sync {
    let base_filter = |env_filter: String| {
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(env_filter))
    };

    let console_layer = tracing_subscriber::fmt::layer()
        .pretty()
        .with_ansi(true)
        .with_timer(GulfiTimer::new())
        .with_span_events(FmtSpan::NONE)
        .with_line_number(false)
        .with_file(false)
        .with_line_number(false)
        // .with_target(false)
        .with_filter(
            base_filter(env_filter.clone())
                .add_directive("opentelemetry_sdk:off".parse().expect("should build")),
        );

    let mut headers = HashMap::new();
    headers.insert(
        "x-honeycomb-team".to_owned(),
        configuration
            .tracer_provider
            .api_key
            .expose_secret()
            .to_string(),
    );

    let telemetry_layer = match SpanExporterBuilder::default()
        .with_http()
        .with_protocol(configuration.tracer_provider.protocol)
        .with_headers(headers)
        .with_endpoint(configuration.tracer_provider.endpoint.clone())
        .build()
    {
        Ok(otlp_exporter) => {
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

            let tracer = provider.tracer("gulfi_server");

            let telemetry_filter = base_filter(env_filter)
                .add_directive("embed=trace".parse().unwrap())
                .add_directive("gen_embeddings=trace".parse().unwrap())
                .add_directive("auth=trace".parse().unwrap())
                .add_directive("favorites=trace".parse().unwrap())
                .add_directive("history=trace".parse().unwrap())
                .add_directive("bg_task=trace".parse().unwrap())
                .add_directive("request=off".parse().unwrap())
                .add_directive("opentelemetry_sdk=off".parse().unwrap());

            Some(
                tracing_opentelemetry::layer()
                    .with_tracer(tracer)
                    .with_filter(telemetry_filter),
            )
        }
        Err(err) => {
            tracing::warn!(
                target: "telemetry",
                error = %err,
                "Telemetry exporter disabled"
            );
            None
        }
    };

    Registry::default()
        .with(console_layer)
        .with(telemetry_layer)
        .with(ErrorLayer::default())
}

pub fn init_subscriber(subscriber: impl Subscriber + Send + Sync) {
    tracing::subscriber::set_global_default(subscriber).expect("Failed to set subscriber");
    LogTracer::init().expect("Failed to set logger");
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
