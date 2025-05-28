use chrono::NaiveDateTime;
use eyre::eyre;
use rusqlite::{Row, params};
use std::collections::HashMap;

use crate::into_http::HttpError;
use axum::{
    Json,
    extract::{Path, Query, State},
};
use http::StatusCode;
use rusqlite::Connection;
use tracing::{debug, info_span};

use crate::{
    HistorialView,
    search::SearchStrategy,
    startup::ServerState,
    views::{HistorialFullView, HistorialParams},
};

#[axum::debug_handler]
#[tracing::instrument(name = "history.fetch_summary", skip(app))]
pub async fn historial_summary(
    Path(doc): Path<String>,
    State(app): State<ServerState>,
) -> eyre::Result<Json<Vec<HistorialView>>, HttpError> {
    let doc_exists = { app.documents.iter().any(|document| document.name == doc) };

    if !doc_exists {
        return Err(HttpError::MissingDocument {
            msg: format!("Document '{doc}' doesn't exists."),
        });
    }

    let conn = {
        let acquire_span = info_span!("conn.acquiring");
        let _guard = acquire_span.enter();
        app.pool.acquire().await?
    };

    let result = get_historial(
        &conn,
        "select id, query from historial where doc = :doc order by timestamp desc",
        |row| {
            let id: u64 = row.get(0).unwrap_or_default();
            let query: String = row.get(1).unwrap_or_default();

            let data = HistorialView::new(id, query);

            Ok(data)
        },
        doc,
    )?;

    Ok(Json(result))
}

#[axum::debug_handler]
#[tracing::instrument(name = "history.fetch_detailed", skip(app))]
pub async fn historial_detailed(
    Path(doc): Path<String>,
    State(app): State<ServerState>,
) -> eyre::Result<Json<Vec<HistorialFullView>>, HttpError> {
    let doc_exists = { app.documents.iter().any(|document| document.name == doc) };

    if !doc_exists {
        return Err(HttpError::MissingDocument {
            msg: format!("Document '{doc}' doesn't exists."),
        });
    }

    let conn = {
        let acquire_span = info_span!("conn.acquiring");
        let _guard = acquire_span.enter();
        app.pool.acquire().await?
    };

    let result = get_historial(
        &conn,
        "select id, query, strategy, peso_fts, peso_semantic, neighbors, timestamp from historial where doc = :doc order by timestamp desc",
        |row| {
            let id: u64 = row.get(0).unwrap_or_default();
            let query: String = row.get(1).unwrap_or_default();
            let strategy: SearchStrategy = row.get(2).unwrap_or_default();
            let peso_fts: f32 = row.get(3).unwrap_or_default();
            let peso_semantic: f32 = row.get(4).unwrap_or_default();
            let neighbors: u64 = row.get(5).unwrap_or_default();
            let timestamp_str: String = row.get(6).unwrap_or_default();

            let timestamp = NaiveDateTime::parse_from_str(&timestamp_str, "%Y-%m-%d %H:%M:%S")
                .unwrap_or_else(|_| NaiveDateTime::default());

            let proper_str = format!("query:{query}");
            let query = gulfi_query::Query::parse(&proper_str).expect("the query is malformed");

            let search_str = query.query;

            let filters = query.constraints.map(|map| {
                map.into_iter()
                    .flat_map(|(field, constraints)| {
                        constraints
                            .into_iter()
                            .map(move |constraint| format!("{field} {constraint}"))
                    })
                    .collect::<Vec<_>>()
                    .join(",")
            });

            let historial_params = HistorialParams::new(
                filters,
                strategy,
                peso_fts,
                peso_semantic,
                neighbors,
                timestamp,
            );

            let data = HistorialFullView::new(id, search_str, historial_params);

            Ok(data)
        },
        doc,
    )?;

    Ok(Json(result))
}

#[tracing::instrument(skip(app), name = "history.deleting")]
pub async fn delete_historial(
    Path(doc): Path<String>,
    State(app): State<ServerState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<StatusCode, HttpError> {
    let doc_exists = { app.documents.iter().any(|document| document.name == doc) };

    if !doc_exists {
        return Err(HttpError::MissingDocument {
            msg: format!("Document '{doc}' doesn't exists."),
        });
    }

    let conn = {
        let acquire_span = info_span!("conn.acquiring");
        let _guard = acquire_span.enter();
        app.pool.acquire().await?
    };

    let mut statement = conn.prepare("delete from historial where query = ? and doc = ?")?;

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

    statement.execute(params![query, doc])?;

    Ok(StatusCode::OK)
}

fn get_historial<T, U>(
    conn: &Connection,
    query: &str,
    f: U,
    doc: String,
) -> Result<Vec<T>, HttpError>
where
    U: Fn(&Row) -> Result<T, rusqlite::Error>,
{
    let mut statement = conn.prepare_cached(query)?;

    let rows = statement
        .query_map([doc], |row| f(row))?
        .collect::<Result<Vec<T>, _>>()?;

    Ok(rows)
}
