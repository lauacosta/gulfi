use eyre::eyre;
use rusqlite::{Row, params};
use std::collections::HashMap;

use axum::{
    Json,
    extract::{Query, State},
};
use gulfi_common::HttpError;
use http::StatusCode;
use rusqlite::Connection;
use tracing::debug;

use crate::{
    HistorialView, Sexo, search::SearchStrategy, startup::AppState, views::HistorialFullView,
};

#[axum::debug_handler]
#[tracing::instrument(name = "Consultando el historial")]
pub async fn historial(
    State(app): State<AppState>,
) -> eyre::Result<Json<Vec<HistorialView>>, HttpError> {
    let db = Connection::open(app.db_path)
        .expect("Deberia ser un path valido a una base de datos SQLite");

    let result = get_historial(
        &db,
        "select id, query from historial order by timestamp desc",
        |row| {
            let id: u64 = row.get(0).unwrap_or_default();
            let query: String = row.get(1).unwrap_or_default();

            let data = HistorialView::new(id, query);

            Ok(data)
        },
    )?;

    #[cfg(debug_assertions)]
    dbg!("{:?}", &result);

    Ok(Json(result))
}

#[axum::debug_handler]
#[tracing::instrument(name = "Consultando el historial")]
pub async fn historial_full(
    State(app): State<AppState>,
) -> eyre::Result<Json<Vec<HistorialFullView>>, HttpError> {
    let db = Connection::open(app.db_path)
        .expect("Deberia ser un path valido a una base de datos SQLite");

    let result = get_historial(
        &db,
        "select id, query, strategy, sexo, edad_min, edad_max, peso_fts, peso_semantic, neighbors from historial order by timestamp desc",
        |row| {
            let id: u64 = row.get(0).unwrap_or_default();
            let query: String = row.get(1).unwrap_or_default();
            let strategy: SearchStrategy = row.get(2).unwrap_or_default();
            let sexo: Sexo = row.get(3).unwrap_or_default();
            let edad_min: u64 = row.get(4).unwrap_or_default();
            let edad_max: u64 = row.get(5).unwrap_or_default();
            let peso_fts: f32 = row.get(6).unwrap_or_default();
            let peso_semantic: f32 = row.get(7).unwrap_or_default();
            let neighbors: u64 = row.get(8).unwrap_or_default();

            let data = HistorialFullView::new(
                id,
                query,
                strategy,
                sexo,
                edad_min,
                edad_max,
                peso_fts,
                peso_semantic,
                neighbors,
            );

            Ok(data)
        },
    )?;

    #[cfg(debug_assertions)]
    dbg!("{:?}", &result);

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

fn get_historial<T, U>(db: &Connection, query: &str, f: U) -> Result<Vec<T>, HttpError>
where
    U: Fn(&Row) -> Result<T, rusqlite::Error>,
{
    let mut statement = db.prepare(query)?;

    let rows = statement
        .query_map([], |row| f(row))?
        .collect::<Result<Vec<T>, _>>()?;

    Ok(rows)
}
