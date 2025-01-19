use axum::{Extension, extract::State};
use gulfi_common::SearchResult;
use tracing::{debug, instrument};

use crate::{routes::Params, routes::SearchExtractor, startup::AppState};

#[axum::debug_handler]
#[instrument(name = "Realizando la búsqueda", skip(app, client))]
pub async fn search(
    SearchExtractor(params): SearchExtractor<Params>,
    State(app): State<AppState>,
    client: Extension<reqwest::Client>,
) -> SearchResult {
    debug!(?params);
    params.strategy.search(&app.db_path, &client, params).await
}
