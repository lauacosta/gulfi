use crate::into_http::SearchResult;
use axum::{Extension, extract::State};
use tracing::debug;

use crate::{
    SearchStrategy, extractors::SearchExtractor, search::SearchParams, startup::ServerState,
};

#[axum::debug_handler]
pub async fn search(
    SearchExtractor(params): SearchExtractor<SearchParams>,
    State(app): State<ServerState>,
    client: Extension<reqwest::Client>,
) -> SearchResult {
    debug!(?params);

    SearchStrategy::search(params.strategy, &app, &client, params).await
}
