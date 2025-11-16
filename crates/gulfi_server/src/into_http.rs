use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use chrono::Local;
use color_eyre::Report;
use gulfi_query::ParsingError;
use serde_json::json;
use std::{fmt, io::Write};
use termcolor::{ColorChoice, StandardStream};
use tracing::error;

use crate::startup::CacheError;

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
    AuthError {
        msg: String,
        err: String,
    },
    MissingDocument {
        msg: String,
    },
    Internal {
        err: String,
    },
    BadRequest {
        message: String,
        valid_fields: Vec<String>,
        invalid_fields: Vec<String>,
    },
    Parsing(ParsingError),
}

impl HttpError {
    pub fn from_report(err: Report) -> Self {
        error!("HTTP handler error: {}", err.root_cause());

        if let Some(bt) = err
            .context()
            .downcast_ref::<color_eyre::Handler>()
            .and_then(|h| h.backtrace())
        {
            error!("Backtrace:");
            let mut stream = StandardStream::stderr(ColorChoice::Always);
            let _ = writeln!(&mut stream, "{bt:?}");
        } else {
            error!("No Backtrace");
        }

        let mut stream = StandardStream::stderr(ColorChoice::Always);
        let _ = writeln!(&mut stream, "{err}");

        HttpError::Internal {
            err: err.to_string(),
        }
    }

    pub fn bad_request(
        message: impl Into<String>,
        valid_fields: Vec<String>,
        invalid_fields: Vec<String>,
    ) -> Self {
        HttpError::BadRequest {
            message: message.into(),
            valid_fields,
            invalid_fields,
        }
    }

    pub fn missing_document(message: impl Into<String>) -> Self {
        HttpError::MissingDocument {
            msg: message.into(),
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
impl_from!(rusqlite::Error);
impl_from!(CacheError);
impl_from!(gulfi_ingest::pool::PoolError);

impl IntoResponse for HttpError {
    fn into_response(self) -> Response {
        let date = Local::now().to_rfc3339();
        match self {
            HttpError::MissingDocument { msg } => (
                StatusCode::BAD_REQUEST,
                Json(json!( { "msg":msg, "date": date } )),
            )
                .into_response(),

            HttpError::AuthError { msg, err } => (
                StatusCode::BAD_REQUEST,
                Json(json!( { "msg":msg, "err": err, "date": date } )),
            )
                .into_response(),

            HttpError::Internal { err } => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!( { "err":err, "date":date })),
            )
                .into_response(),
            HttpError::BadRequest {
                message,
                valid_fields,
                invalid_fields,
            } => (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "err": message,
                    "type": "invalid_fields",
                    "valid_fields": valid_fields,
                    "invalid_fields": invalid_fields,
                    "date": date
                })),
            )
                .into_response(),
            HttpError::Parsing(e) => match e {
                ParsingError::InvalidToken(_) => (
                    StatusCode::BAD_REQUEST,
                    Json(json!({
                        "err": e.to_string(),
                        "type": "invalid_token",
                        "date": date

                    })),
                )
                    .into_response(),
                ParsingError::MissingQuery | ParsingError::EmptyInput => (
                    StatusCode::BAD_REQUEST,
                    Json(json!({
                        "err": e.to_string(),
                        "type": "parsing_error",
                        "date": date
                    })),
                )
                    .into_response(),

                ParsingError::MissingValue(c) | ParsingError::MissingKey(c) => (
                    StatusCode::BAD_REQUEST,
                    Json(json!({
                        "err": e.to_string(),
                        "type": "parsing_error",
                        "token": c.to_string(),
                        "date": date
                    })),
                )
                    .into_response(),
            },
        }
    }
}

impl From<ParsingError> for HttpError {
    fn from(e: ParsingError) -> Self {
        HttpError::Parsing(e)
    }
}
impl From<argon2::password_hash::Error> for HttpError {
    fn from(err: argon2::password_hash::Error) -> Self {
        Self::from_report(eyre::eyre!("argon2 error: {:?}", err))
    }
}

impl std::error::Error for HttpError {}

impl fmt::Display for HttpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let msg = match self {
            HttpError::AuthError { msg, err } => format!("{msg}{err}"),
            HttpError::MissingDocument { msg } => msg.to_owned(),
            HttpError::Internal { err } => err.to_owned(),
            HttpError::BadRequest { message, .. } => message.to_owned(),
            HttpError::Parsing(parsing_error) => parsing_error.to_string(),
        };
        write!(f, "HttpError: {}", msg)
    }
}
