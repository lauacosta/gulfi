use axum::{async_trait, extract::FromRequestParts};
use crate::into_http::HttpError;
use http::{Uri, request::Parts};
use serde::de::DeserializeOwned;

pub struct SearchExtractor<T>(pub T);

impl<T> SearchExtractor<T>
where
    T: DeserializeOwned,
{
    pub fn try_from_uri(value: &Uri) -> Result<Self, HttpError> {
        let query = value.query().unwrap_or_default();
        let params = serde_urlencoded::from_str(query)?;
        Ok(SearchExtractor(params))
    }
}

#[async_trait]
impl<T, S> FromRequestParts<S> for SearchExtractor<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = HttpError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        Self::try_from_uri(&parts.uri)
    }
}
