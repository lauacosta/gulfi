use axum::Json;
use axum::extract::Query;
use chrono::NaiveDateTime;
use eyre::{Result, eyre};
use serde::Serialize;
use std::collections::HashMap;
use tracing::debug;

use axum::extract::State;
use gulfi_common::HttpError;
use http::StatusCode;
use rusqlite::Connection;
use rusqlite::params;
use serde::Deserialize;

use crate::startup::AppState;
use crate::views::FavoritosView;
use crate::views::ResultadosView;

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
pub struct FavParams {
    nombre: String,
    data: String,
    busquedas: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct FavoritesReponse {
    query: String,
    strategy: String,
}

#[tracing::instrument(skip(app, payload), name = "añadiendo busqueda a favoritos")]
pub async fn add_favoritos(
    State(app): State<AppState>,
    Json(payload): Json<FavParams>,
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

fn get_favoritos(db: &Connection) -> Result<FavoritosView, HttpError> {
    let mut statement = db.prepare(
        "select id, nombre, data, timestamp, busquedas, tipos from favoritos order by timestamp desc",
    )?;

    let rows = statement
        .query_map([], |row| {
            let id: u64 = row.get(0).unwrap_or_default();
            let nombre: String = row.get(1).unwrap_or_default();
            let data: String = row.get(2).unwrap_or_default();
            let timestamp_str: String = row.get(3).unwrap_or_default();
            let bus: String = row.get(4).unwrap_or_default();
            let tipo: String = row.get(5).unwrap_or_default();

            let timestamp = NaiveDateTime::parse_from_str(&timestamp_str, "%Y-%m-%d %H:%M:%S")
                .unwrap_or_else(|_| Default::default());

            let busquedas: Vec<String> =
                serde_json::from_str(&bus).expect("busquedas tendria que poder ser serializado");

            let tipos: Vec<String> =
                serde_json::from_str(&tipo).expect("tipos tendria que poder ser serializado");

            let data = ResultadosView::new(id, nombre, data, tipos, timestamp, busquedas);

            Ok(data)
        })?
        .collect::<Result<Vec<ResultadosView>, _>>()?;

    Ok(FavoritosView { favoritos: rows })
}
