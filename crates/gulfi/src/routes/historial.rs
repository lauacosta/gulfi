use eyre::eyre;
use rusqlite::params;
use std::collections::HashMap;

use axum::{
    Json,
    extract::{Query, State},
};
use gulfi_common::HttpError;
use gulfi_ui::Historial;
use http::StatusCode;
use rusqlite::Connection;
use tracing::debug;

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

#[tracing::instrument(skip(app), name = "borrando busqueda del historial")]
pub async fn delete_historial(
    State(app): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<StatusCode, HttpError> {
    let db = Connection::open(app.db_path)
        .expect("Deberia ser un path valido a una base de datos SQLite");
    let mut statement = db.prepare("delete from historial where query = ?")?;

    let query = {
        if let Some(query) = params.get("query") {
            debug!(?query);
            query
        } else {
            return Err(HttpError::from_report(eyre!(
                "No se encuentra el parametro 'query'."
            )));
        }
    };

    statement.execute(params![query])?;

    Ok(StatusCode::OK)
}
