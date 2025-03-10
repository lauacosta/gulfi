use axum::{Extension, extract::State};
use gulfi_common::SearchResult;
use tracing::{debug, instrument};

use crate::{
    routes::{Params, SearchExtractor, SearchStrategy},
    startup::AppState,
};

#[axum::debug_handler]
#[instrument(name = "Realizando la búsqueda", skip(app, client, params))]
pub async fn search(
    SearchExtractor(params): SearchExtractor<Params>,
    State(app): State<AppState>,
    client: Extension<reqwest::Client>,
) -> SearchResult {
    debug!(?params);

    SearchStrategy::search(params.strategy, &app.db_path, &client, params).await
}
