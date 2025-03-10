use axum::Json;
use axum::extract::Query;
use eyre::{Result, eyre};
use serde::Serialize;
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

#[derive(Debug, Clone, Default, Serialize)]
pub struct Resultados {
    pub id: u64,
    pub nombre: String,
    pub data: String,
    pub busquedas: Vec<(String, String)>,
    pub fecha: String,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct FavoritosClient {
    pub favoritos: Vec<Resultados>,
}

pub async fn favoritos(State(app): State<AppState>) -> Result<Json<FavoritosClient>, HttpError> {
    let db = Connection::open(app.db_path)
        .expect("Deberia ser un path valido a una base de datos SQLite");
    let favoritos = get_favoritos(&db)?;
    let mut results = Vec::with_capacity(favoritos.favoritos.len());
    for f in &favoritos.favoritos {
        let id = f.id;
        let nombre = f.nombre.clone();
        let data = f.data.clone();
        let busquedas = f
            .busquedas
            .clone()
            .into_iter()
            .zip(f.tipos.clone().into_iter())
            .collect();
        let fecha = f.fecha.clone();

        results.push(Resultados {
            id,
            nombre,
            data,
            busquedas,
            fecha,
        });
    }

    Ok(Json(FavoritosClient { favoritos: results }))
}

#[derive(Deserialize, Debug)]
pub struct Params {
    nombre: String,
    data: String,
    busquedas: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct FavoritesReponse {
    query: String,
    strategy: String,
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
    let resp: Vec<FavoritesReponse> = serde_json::from_str(&payload.busquedas)?;
    let queries = serde_json::to_string(
        &resp
            .iter()
            .map(|x| x.query.clone())
            .collect::<Vec<String>>(),
    )?;

    let tipos = serde_json::to_string(
        &resp
            .iter()
            .map(|x| x.strategy.clone())
            .collect::<Vec<String>>(),
    )?;

    let mut statement = db.prepare(
        "insert into favoritos (nombre, data, busquedas,tipos, timestamp) values (?,?,?,?,datetime('now', 'localtime'))",
    )?;

    statement.execute(params![nombre, data, queries, tipos])?;

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
