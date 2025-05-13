use axum::Json;
use axum::extract::Path;
use axum::extract::Query;
use chrono::NaiveDateTime;
use eyre::{Result, eyre};
use serde::Serialize;
use serde_json::json;
use std::collections::HashMap;
use std::iter::zip;
use tracing::debug;

use crate::into_http::HttpError;
use axum::extract::State;
use http::StatusCode;
use rusqlite::Connection;
use rusqlite::params;
use serde::Deserialize;

use crate::startup::AppState;

#[derive(Debug, Clone, Default, Serialize)]
struct Resultados {
    id: u64,
    nombre: String,
    data: String,
    busquedas: Vec<(String, String)>,
    fecha: String,
}

pub async fn favoritos(
    Path(doc): Path<String>,
    State(app): State<AppState>,
) -> Result<Json<serde_json::Value>, HttpError> {
    let db = Connection::open(app.db_path)
        .expect("Deberia ser un path valido a una base de datos SQLite");

    let favoritos = {
        let mut statement = db.prepare(
        "select id, nombre, data, timestamp, busquedas, tipos from favoritos where doc = :doc order by timestamp desc",
    )?;

        statement
            .query_map([doc], |row| {
                let id: u64 = row.get(0).unwrap_or_default();
                let nombre: String = row.get(1).unwrap_or_default();
                let data: String = row.get(2).unwrap_or_default();
                let timestamp_str: String = row.get(3).unwrap_or_default();

                let busquedas: String = row.get(4).unwrap_or_default();
                let tipos: String = row.get(5).unwrap_or_default();

                let busquedas: Vec<String> = serde_json::from_str(&busquedas).map_err(|e| {
                    tracing::error!(
                        "Error al deserializar el string de busquedas a Vec<String>: {e}"
                    );

                    rusqlite::Error::ExecuteReturnedResults
                })?;
                let tipos: Vec<String> = serde_json::from_str(&tipos).map_err(|e| {
                    tracing::error!("Error al deserializar el string de tipos a Vec<String>: {e}");

                    rusqlite::Error::ExecuteReturnedResults
                })?;

                let timestamp = NaiveDateTime::parse_from_str(&timestamp_str, "%Y-%m-%d %H:%M:%S")
                    .unwrap_or_else(|_| Default::default());

                let busqueda_con_tipo: Vec<(String, String)> = zip(busquedas, tipos).collect();

                let data = Resultados {
                    id,
                    nombre,
                    data,
                    busquedas: busqueda_con_tipo,
                    fecha: timestamp.format("%b %d, %Y %H:%M").to_string(),
                };

                Ok(data)
            })?
            .collect::<Result<Vec<Resultados>, _>>()
    }?;

    Ok(Json(json! ({ "favoritos": favoritos})))
}

#[derive(Deserialize, Debug)]
pub struct FavParams {
    nombre: String,
    data: Vec<String>,
    busquedas: Vec<Busquedas>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Busquedas {
    query: String,
    strategy: String,
}

#[tracing::instrument(skip(app), name = "añadiendo busqueda a favoritos")]
pub async fn add_favoritos(
    Path(doc): Path<String>,
    State(app): State<AppState>,
    Json(payload): Json<FavParams>,
) -> Result<(StatusCode, String), HttpError> {
    let db = Connection::open(app.db_path)
        .expect("Deberia ser un path valido a una base de datos SQLite");

    let nombre = payload.nombre.replace(|c: char| c.is_whitespace(), "_");
    let data = payload.data.join(", ");
    let busquedas = payload.busquedas;

    let queries = busquedas
        .iter()
        .map(|x| {
            gulfi_query::Query::parse(&format!("query: {}", &x.query)).map(|parsed| parsed.query)
        })
        .collect::<Result<Vec<String>, _>>()?;

    let queries = serde_json::to_string(&queries)?;

    let tipos = serde_json::to_string(
        &busquedas
            .into_iter()
            .map(|x| x.strategy)
            .collect::<Vec<String>>(),
    )?;

    let mut statement = db.prepare(
        "insert into favoritos (nombre, data, doc, busquedas,tipos, timestamp) values (?,?,?,?,?,datetime('now', 'localtime'))",
    )?;

    statement.execute(params![nombre, data, doc, queries, tipos])?;

    Ok((
        StatusCode::OK,
        "Busqueda añadida a favoritos exitosamente!".to_string(),
    ))
}

#[tracing::instrument(skip(app), name = "borrando busqueda de favoritos")]
pub async fn delete_favoritos(
    Path(doc): Path<String>,
    State(app): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<StatusCode, HttpError> {
    let db = Connection::open(app.db_path)
        .expect("Deberia ser un path valido a una base de datos SQLite");
    let mut statement = db.prepare("delete from favoritos where nombre = ? and doc = ?")?;

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

    statement.execute(params![nombre, doc])?;

    Ok(StatusCode::OK)
}
