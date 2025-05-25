use color_eyre::owo_colors::OwoColorize;
use std::fmt;

pub struct GulfiTimer;

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
