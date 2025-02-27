use axum::Json;
use axum::extract::Query;
use eyre::{Result, eyre};
use gulfi_ui::Favoritos;
use std::collections::HashMap;
use tracing::debug;

use axum::extract::State;
use gulfi_common::HttpError;
use gulfi_sqlite::get_favoritos;
use http::StatusCode;
use rusqlite::Connection;
use rusqlite::params;
use serde::Deserialize;

use crate::startup::AppState;

pub async fn favoritos(State(app): State<AppState>) -> Result<Json<Favoritos>, HttpError> {
    let db = Connection::open(app.db_path)
        .expect("Deberia ser un path valido a una base de datos SQLite");
    let favoritos = get_favoritos(&db)?;

    Ok(Json(favoritos))
}

#[derive(Deserialize, Debug)]
pub struct Params {
    nombre: String,
    data: String,
    busquedas: String,
}

#[tracing::instrument(skip(app), name = "añadiendo busqueda a favoritos")]
pub async fn add_favoritos(
    State(app): State<AppState>,
    Json(payload): Json<Params>,
) -> Result<(StatusCode, String), HttpError> {
    let db = Connection::open(app.db_path)
        .expect("Deberia ser un path valido a una base de datos SQLite");
    let nombre = payload.nombre.replace(|c: char| c.is_whitespace(), "_");
    let data = payload.data;
    let busquedas = payload.busquedas;

    let mut statement = db.prepare(
        "insert into favoritos (nombre, data, busquedas, timestamp) values (?,?,?,datetime('now', 'localtime'))",
    )?;

    statement.execute(params![nombre, data, busquedas])?;

    Ok((
        StatusCode::OK,
        "Busqueda añadida a favoritos exitosamente!".to_string(),
    ))
}

#[tracing::instrument(skip(app), name = "borrando busqueda de favoritos")]
pub async fn delete_favoritos(
    State(app): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<StatusCode, HttpError> {
    let db = Connection::open(app.db_path)
        .expect("Deberia ser un path valido a una base de datos SQLite");
    let mut statement = db.prepare("delete from favoritos where nombre = ?")?;

    let nombre = {
        if let Some(nombre) = params.get("nombre") {
            debug!(?nombre);
            nombre
        } else {
            return Err(HttpError::from_report(eyre!(
                "No se encuentra el parametro 'nombre'."
            )));
        }
    };

    statement.execute(params![nombre])?;

    Ok(StatusCode::OK)
}
