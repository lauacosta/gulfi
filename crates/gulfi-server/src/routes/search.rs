use axum::{Extension, extract::State};
use crate::into_http::SearchResult;
use tracing::{debug, instrument};

use crate::{SearchStrategy, extractors::SearchExtractor, search::SearchParams, startup::AppState};

#[axum::debug_handler]
#[instrument(name = "Realizando la búsqueda", skip(app, client, params))]
pub async fn search(
    SearchExtractor(params): SearchExtractor<SearchParams>,
    State(app): State<AppState>,
    client: Extension<reqwest::Client>,
) -> SearchResult {
    debug!(?params);

    SearchStrategy::search(params.strategy, &app, &client, params).await
}
