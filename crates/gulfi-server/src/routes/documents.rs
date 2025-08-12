use axum::{Json, extract::State};
use gulfi_ingest::Document;

use crate::startup::ServerState;

pub async fn documents(State(app): State<ServerState>) -> Json<Vec<Document>> {
    Json(app.documents)
}
