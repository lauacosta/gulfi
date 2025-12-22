use axum::{Extension, extract::State, response::IntoResponse};
use tracing::debug;

use crate::{
    extractors::SearchExtractor,
    search::{SearchParams, search_stream},
    startup::ServerState,
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
    search_stream(app, client, params).await
}
