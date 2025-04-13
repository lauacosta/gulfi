use axum::{Json, extract::State};
use gulfi_common::Document;

use crate::startup::AppState;

pub async fn documents(State(app): State<AppState>) -> Json<Vec<Document>> {
    Json(app.documents)
}
