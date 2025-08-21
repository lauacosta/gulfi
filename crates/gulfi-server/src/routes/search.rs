use axum::{Extension, extract::State, response::IntoResponse};
use tracing::debug;

use crate::{
    SearchStrategy, extractors::SearchExtractor, search::SearchParams, startup::ServerState,
};

#[axum::debug_handler]
pub async fn search(
    SearchExtractor(params): SearchExtractor<SearchParams>,
    State(app): State<ServerState>,
    Extension(client): Extension<reqwest::Client>,
) -> impl IntoResponse {
    debug!(?params);
    let app = app.clone();
    let client = client.clone();
    SearchStrategy::search_stream(params.strategy, app, client, params).await
}
