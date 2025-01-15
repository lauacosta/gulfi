use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use chrono::Local;
use std::io::Write;
use termcolor::{ColorChoice, StandardStream};
use tracing::error;

use crate::Fallback;

pub type SearchResult = Result<Response, HttpError>;

pub trait IntoHttp {
    fn into_http(self) -> SearchResult;
}

impl<T: IntoResponse> IntoHttp for T {
    fn into_http(self) -> SearchResult {
        Ok(self.into_response())
    }
}

#[derive(Debug)]
pub enum HttpError {
    Internal { err: String },
}

impl HttpError {
    fn from_report(err: color_eyre::Report) -> Self {
        error!("HTTP handler error: {}", err.root_cause());

        if let Some(bt) = err
            .context()
            .downcast_ref::<color_eyre::Handler>()
            .and_then(|h| h.backtrace())
        {
            error!("Backtrace:");
            let mut stream = StandardStream::stderr(ColorChoice::Always);
            let _ = writeln!(&mut stream, "{:?}", bt);
        } else {
            error!("No Backtrace");
        }

        let mut stream = StandardStream::stderr(ColorChoice::Always);
        let _ = writeln!(&mut stream, "{}", err);

        HttpError::Internal {
            err: err.to_string(),
        }
    }
}

macro_rules! impl_from {
    ($from:ty) => {
        impl From<$from> for HttpError {
            fn from(err: $from) -> Self {
                let report = color_eyre::Report::from(err);
                Self::from_report(report)
            }
        }
    };
}

impl_from!(std::io::Error);
impl_from!(serde_urlencoded::de::Error);
impl_from!(serde_json::Error);
impl_from!(rinja::Error);
impl_from!(rusqlite::Error);

impl IntoResponse for HttpError {
    fn into_response(self) -> Response {
        let date = Local::now().to_rfc3339();
        match self {
            HttpError::Internal { err } => {
                (StatusCode::INTERNAL_SERVER_ERROR, Fallback { err, date }).into_response()
            }
        }
    }
}
