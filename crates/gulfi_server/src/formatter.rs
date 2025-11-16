use std::{fmt, time::Duration};

use color_eyre::owo_colors::OwoColorize;
use tower_http::trace::OnResponse;
use tracing::Level;

#[derive(Clone)]
pub struct ColoredOnResponse {
    level: Level,
    include_headers: bool,
}

impl Default for ColoredOnResponse {
    fn default() -> Self {
        Self::new()
    }
}

impl ColoredOnResponse {
    pub fn new() -> Self {
        Self {
            level: Level::INFO,
            include_headers: false,
        }
    }

    pub fn level(mut self, level: Level) -> Self {
        self.level = level;
        self
    }

    pub fn include_headers(mut self, include: bool) -> Self {
        self.include_headers = include;
        self
    }
}

struct Latency {
    duration: Duration,
}

impl Latency {
    fn new(duration: Duration) -> Self {
        Self { duration }
    }

    fn best_unit_and_value(&self) -> (f64, &'static str) {
        let nanos = self.duration.as_nanos();

        if nanos >= 1_000_000_000 {
            (self.duration.as_secs_f64(), "s")
        } else if nanos >= 1_000_000 {
            (self.duration.as_millis() as f64, "ms")
        } else if nanos >= 1_000 {
            (self.duration.as_micros() as f64, "μs")
        } else {
            (nanos as f64, "ns")
        }
    }
}

impl fmt::Display for Latency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (value, unit) = self.best_unit_and_value();

        write!(f, "{value:.3} {unit}")
    }
}

macro_rules! event_dynamic_lvl {
    ( $(target: $target:expr,)? $(parent: $parent:expr,)? $lvl:expr, $($tt:tt)* ) => {
        match $lvl {
            tracing::Level::ERROR => {
                tracing::event!(
                    $(target: $target,)?
                    $(parent: $parent,)?
                    tracing::Level::ERROR,
                    $($tt)*
                );
            }
            tracing::Level::WARN => {
                tracing::event!(
                    $(target: $target,)?
                    $(parent: $parent,)?
                    tracing::Level::WARN,
                    $($tt)*
                );
            }
            tracing::Level::INFO => {
                tracing::event!(
                    $(target: $target,)?
                    $(parent: $parent,)?
                    tracing::Level::INFO,
                    $($tt)*
                );
            }
            tracing::Level::DEBUG => {
                tracing::event!(
                    $(target: $target,)?
                    $(parent: $parent,)?
                    tracing::Level::DEBUG,
                    $($tt)*
                );
            }
            tracing::Level::TRACE => {
                tracing::event!(
                    $(target: $target,)?
                    $(parent: $parent,)?
                    tracing::Level::TRACE,
                    $($tt)*
                );
            }
        }
    };
}

impl<B> OnResponse<B> for ColoredOnResponse {
    fn on_response(self, response: &http::Response<B>, latency: Duration, _: &tracing::Span) {
        let latency = Latency::new(latency);

        let response_headers = self
            .include_headers
            .then(|| tracing::field::debug(response.headers()));

        let status = response.status();
        let colored_status = if status.is_success() {
            format!("{}", status.bright_green().bold())
        } else if status.is_client_error() {
            format!("{}", status.bright_yellow().bold())
        } else if status.is_server_error() {
            format!("{}", status.bright_red().bold())
        } else {
            format!("{}", status.bright_cyan().bold())
        };

        event_dynamic_lvl!(
            self.level,
            latency = %format!("{}", latency.bright_blue().bold()),
            status = %colored_status,
            response_headers,
            "request procesado"
        );
    }
}
