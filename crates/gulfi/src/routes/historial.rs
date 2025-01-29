use axum::{Json, extract::State};
use gulfi_common::HttpError;
use gulfi_ui::Historial;
use rusqlite::Connection;

use crate::startup::AppState;

#[axum::debug_handler]
#[tracing::instrument(name = "Consultando el historial")]
pub async fn historial(
    State(app): State<AppState>,
) -> eyre::Result<Json<Vec<Historial>>, HttpError> {
    let db = Connection::open(app.db_path)
        .expect("Deberia ser un path valido a una base de datos SQLite");
    let result = gulfi_sqlite::get_historial(&db)?;
    Ok(Json(result))
}
