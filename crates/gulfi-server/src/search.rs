#![allow(clippy::too_many_lines)]
use crate::{
    bg_tasks::WriteJob,
    into_http::{HttpError, IntoHttp, SearchResult},
};
use axum::Json;
use eyre::Report;
use gulfi_query::{
    Constraint::{self, Exact, GreaterThan, LesserThan},
    Query,
};
use std::{collections::HashMap, fmt::Write, sync::Arc};

use reqwest::Client;
use rusqlite::{
    Row, ToSql,
    types::{FromSql, FromSqlError, ToSqlOutput, ValueRef},
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use thiserror::Error;
use tracing::{Span, debug, info, info_span, instrument};
use zerocopy::IntoBytes;

use crate::startup::ServerState;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default)]
pub enum SearchStrategy {
    #[default]
    Fts,
    Semantic,
    ReciprocalRankFusion,
}

#[derive(Deserialize, Debug, Clone)]
pub struct SearchParams {
    #[serde(rename = "query")]
    pub search_str: String,
    pub document: String,
    pub strategy: SearchStrategy,
    pub peso_fts: f32,
    pub peso_semantic: f32,
    #[serde(rename = "k")]
    pub k_neighbors: u64,
}

type QueryResult = Result<(Vec<String>, Vec<Vec<String>>, usize), HttpError>;
impl SearchStrategy {
    #[instrument(name = "searching", skip(self, state, client, params), fields(source = tracing::field::Empty))]
    pub async fn search(
        self,
        state: &ServerState,
        client: &Client,
        params: SearchParams,
    ) -> SearchResult {
        let span = Span::current();

        let document = state
            .documents
            .iter()
            .find(|x| x.name.to_lowercase() == params.document.to_lowercase())
            .ok_or_else(|| {
                HttpError::missing_document(format!("Document {} not found", params.document))
            })?;

        let weight_vec = params.peso_semantic / 100.0;
        let weight_fts: f32 = params.peso_fts / 100.0;
        let rrf_k: i64 = 60;
        let k = params.k_neighbors;

        let string = format!("query: {}", params.search_str);
        let query = Query::parse(&string).map_err(HttpError::from)?;
        dbg!("{:#?}", &query);

        let (valid_fields, invalid_fields) = {
            let mut invalid = Vec::new();
            let fields: Vec<_> = document
                .fields
                .iter()
                .filter(|v| !v.vec_input)
                .map(|v| v.name.clone())
                .collect();

            if let Some(constraints) = &query.constraints {
                for k in constraints.keys() {
                    if !fields.contains(k) {
                        invalid.push(k.clone());
                    }
                }
            }

            (fields, invalid)
        };

        if !invalid_fields.is_empty() {
            return Err(HttpError::bad_request(
                "You are looking for fields that don't exist in the document.".to_owned(),
                valid_fields,
                invalid_fields,
            ));
        }

        let mut query_emb: Option<Arc<Vec<f32>>> = None;
        match self {
            SearchStrategy::Semantic | SearchStrategy::ReciprocalRankFusion => {
                if let Some(cached_embedding) = state.embeddings_cache.get(&query.query).await {
                    query_emb = Some(cached_embedding);
                    span.record("source", "hit");
                } else {
                    let embedding_span = info_span!("embedding.request");
                    let _guard = embedding_span.enter();
                    let embedding = Arc::new(
                        state
                            .embeddings_provider
                            .embed_single(query.query.clone(), client)
                            .await
                            .map_err(|err| {
                                tracing::error!("{err}");
                                HttpError::Internal {
                                    err: "Failed to create query embedding".to_string(),
                                }
                            })?,
                    );

                    state
                        .embeddings_cache
                        .insert(query.query.clone(), embedding.clone())
                        .await;

                    query_emb = Some(embedding);
                    span.record("source", "miss");
                }
            }
            SearchStrategy::Fts => {
                span.record("source", "dynamic");
            }
        };

        debug!(?query);

        let (column_names, table, total_query_count) = {
            let pool = state.pool.clone();
            let conn_handle = {
                let conn_span = info_span!("conn.acquire");
                let _guard = conn_span.enter();
                pool.acquire().await?
            };

            debug!("{:?}", pool.stats());
            debug!("{:?}", pool.corruption_info());

            let query_execution_span = info_span!("query.execution");
            let _guard = query_execution_span.enter();

            match self {
                // I dont use `task::spawn_blocking` here because it proved to just add overhead.
                SearchStrategy::Fts => {
                    let search = {
                        let start = "select rank as score,";
                        let mut fields = String::new();

                        for field in &document.fields {
                            if !field.vec_input {
                                let _ = write!(fields, "{},", field.name);
                            }
                        }

                        let doc_name = document.name.clone();
                        format!(
                            "{start}{fields}  highlight(fts_{doc_name}, 0, '<b style=\"color: green;\">', '</b>') as input, 'fts' as match_type from fts_{doc_name}",
                        )
                    };

                    let (mut conditions, mut binding_values) =
                        build_conditions(query.constraints.as_ref());

                    conditions.push("vec_input match '\"' || :query || '\"' ".to_owned());
                    binding_values.push(&query.query as &dyn ToSql);

                    let where_clause = if conditions.is_empty() {
                        String::new()
                    } else {
                        format!("where {}", conditions.join(" and "))
                    };

                    let sql = format!("{search} {where_clause}");

                    let mut stmt = conn_handle.prepare_cached(&sql)?;

                    let column_names: Vec<String> =
                        stmt.column_names().into_iter().map(String::from).collect();

                    let table = stmt
                        .query_map(&*binding_values, |row| {
                            let mut data = Vec::new();

                            for i in 0..row.as_ref().column_count() {
                                let val = sqlite_value_to_string(row, i)?;
                                data.push(val);
                            }

                            Ok(data)
                        })?
                        .collect::<Result<Vec<Vec<String>>, _>>()?;

                    let count = table.len();

                    (column_names, table, count)
                }
                SearchStrategy::Semantic | SearchStrategy::ReciprocalRankFusion => {
                    let search_strategy = self;
                    let document = document.clone();
                    let query = query.clone();

                    tokio::task::spawn_blocking( move || -> QueryResult  {
                    let result = match search_strategy {
                        SearchStrategy::Semantic => {
                            let embedding = query_emb.ok_or_else(|| HttpError::Internal { err: "failed to create embedding".to_owned()})?;
                            let embedding = embedding.as_bytes();

                            let search = {
                                let start = format!("select vec_{}.distance,", document.name);
                                let mut fields = String::new();

                                for field in &document.fields {
                                    if !field.vec_input {
                                        let _ = write!(fields, " {}.{},", document.name, field.name);
                                    }
                                }

                                let doc_name = document.name.clone();
                                format!(
                                    "{start} {fields} {doc_name}.vec_input as input, 'vec' as match_type from vec_{doc_name} left join {doc_name} on {doc_name}.id = vec_{doc_name}.row_id"
                                )
                            };

                            let (mut conditions, mut binding_values) =
                                build_conditions(query.constraints.as_ref());

                            conditions.push("k = :k".to_owned());
                            binding_values.push(&k as &dyn ToSql);

                            conditions.push("vec_input_embedding match :embedding ".to_owned());
                            binding_values.push(&embedding as &dyn ToSql);

                            let where_clause = if conditions.is_empty() {
                                String::new()
                            } else {
                                format!("where {}", conditions.join(" and "))
                            };

                            let sql = format!("{search} {where_clause}");

                            let mut stmt = conn_handle.prepare_cached(&sql)?;
                            let column_names: Vec<String> =
                                stmt.column_names().into_iter().map(String::from).collect();

                            let table = stmt
                                .query_map(&*binding_values, |row| {
                                    let mut data = Vec::new();

                                    for i in 0..row.as_ref().column_count() {
                                        let val = sqlite_value_to_string(row, i)?;
                                        data.push(val);
                                    }

                                    Ok(data)
                                })?
                                .collect::<Result<Vec<Vec<String>>, _>>()?;

                            let count = table.len();

                            (column_names, table, count)
                        }
                        SearchStrategy::ReciprocalRankFusion => {
                            let embedding = query_emb.ok_or_else(|| HttpError::Internal { err: "failed to create embedding".to_owned()})?;
                            let embedding = embedding.as_bytes();

                            let build_final_query = |conditions: &str| -> String {
                                let doc_name = document.name.clone();
                                let mut fields = String::new();

                                for field in &document.fields {
                                    if !field.vec_input {
                                        let _ = write!(fields, "{doc_name}.{},", field.name);
                                    }
                                }

                                let search_query = format!(
                                "select 
                                    {fields}
                                    {doc_name}.vec_input as input,
                                    vec_matches.rank_number as vec_rank,
                                    fts_matches.rank_number as fts_rank,
                                    (
                                        coalesce(1.0 / (:rrf_k + fts_matches.rank_number), 0.0) * :weight_fts +
                                        coalesce(1.0 / (:rrf_k + vec_matches.rank_number), 0.0) * :weight_vec
                                    ) as combined_rank,
                                    vec_matches.distance as vec_distance,
                                    fts_matches.score as fts_score
                                from fts_matches
                                full outer join vec_matches on vec_matches.row_id = fts_matches.row_id
                                join {doc_name} on {doc_name}.id = coalesce(fts_matches.row_id, vec_matches.row_id)");

                                let base = format!(
                                "with vec_matches as (
                                    select
                                        row_id,
                                        row_number() over (order by distance) as rank_number,
                                        distance
                                    from vec_{doc_name}
                                    where
                                        vec_input_embedding match :embedding
                                        and k = :k
                                ),

                                fts_matches as (
                                    select
                                        rowid as row_id,
                                        row_number() over (order by rank) as rank_number,
                                        rank as score
                                    from fts_{doc_name}
                                    where vec_input match '\"' || :query || '\"'
                                    ),

                                final as ( {search_query} {conditions} order by combined_rank desc) select * from final;"
                            );

                                base
                            };

                            let mut conditions = Vec::new();

                            let mut binding_values: Vec<&dyn ToSql> = vec![
                                &embedding as &dyn ToSql,
                                &k as &dyn ToSql,
                                &query.query as &dyn ToSql,
                                &rrf_k as &dyn ToSql,
                                &weight_fts as &dyn ToSql,
                                &weight_vec as &dyn ToSql,
                            ];

                            //INFO: EL orden en que los campos son cargados en binding_values es importante.
                            //No me fascina pero por ahora no es el mayor de mis problemas.

                            if let Some(contraints) = &query.constraints {
                                for (k, values) in contraints {
                                    for (i, cons) in values.iter().enumerate() {
                                        let param_name = format!(":{k}_{i}");
                                        let condition = match cons {
                                            Exact(_) => {
                                                format!("LOWER({k}) like LOWER('%' || {param_name} || '%')")
                                            }
                                            GreaterThan(_) => format!("{k} > {param_name}"),
                                            LesserThan(_) => format!("{k} < {param_name}"),
                                        };

                                        let value = match cons {
                                            Exact(val) | GreaterThan(val) | LesserThan(val) => val,
                                        };

                                        conditions.push(condition);
                                        binding_values.push(value);
                                    }
                                }
                            }

                            let where_clause = if conditions.is_empty() {
                                String::new()
                            } else {
                                format!("where {}", conditions.join(" and "))
                            };

                            let sql = build_final_query(&where_clause);

                            let mut stmt = conn_handle.prepare_cached(&sql)?;
                            let column_names: Vec<String> =
                                stmt.column_names().into_iter().map(String::from).collect();

                            let column_count = stmt.column_count();
                            let table = stmt
                                .query_map(&*binding_values, |row| {
                                    let mut data = Vec::with_capacity(column_count);

                                    for i in 0..row.as_ref().column_count() {
                                        let val = sqlite_value_to_string(row, i)?;
                                        data.push(val);
                                    }

                                    Ok(data)
                                })?
                                .collect::<Result<Vec<Vec<String>>, _>>()?;

                            let count = table.len();

                            (column_names, table, count)
                        }
                        SearchStrategy::Fts => unreachable!(),
                    };
                    Ok(result)
                    }).await.map_err(|join_err|  {
                        tracing::error!("Database task panicked: {:?}", join_err);
                        HttpError::Internal { err: "Database operation failed".to_owned() }
                    })??
                }
            }
        };

        info!(
            "Busqueda para el query: `{}`, exitosa! {} registros",
            query.query, total_query_count,
        );

        if let Err(e) = state.writer.send(WriteJob::History {
            query: params.search_str,
            doc: params.document,
            strategy: params.strategy,
            peso_fts: params.peso_fts,
            peso_semantic: params.peso_semantic,
            k_neighbors: params.k_neighbors,
        }) {
            tracing::error!("No se pudo enviar a la tarea de escritura: {:?}", e);
        }

        Json(json!({
            "msg": format!("Hay un total de {} resultados", total_query_count),
            "columns": column_names,
            "rows": table,
        }))
        .into_http()
    }
}

impl TryFrom<String> for SearchStrategy {
    type Error = Report;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "fts" => Ok(Self::Fts),
            "semantic_search" => Ok(Self::Semantic),
            "rrf" => Ok(Self::ReciprocalRankFusion),
            // "hkf" => Ok(Self::KeywordFirst),
            // "rrs" => Ok(Self::ReRankBySemantics),
            other => Err(SearchStrategyError::UnsupportedSearchStrategy(other.to_owned()).into()),
        }
    }
}

impl ToSql for SearchStrategy {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        let value = match self {
            SearchStrategy::Fts => "Fts",
            SearchStrategy::Semantic => "Semantic",
            SearchStrategy::ReciprocalRankFusion => "ReciprocalRankFusion",
        };
        Ok(ToSqlOutput::from(value))
    }
}

impl FromSql for SearchStrategy {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        match value {
            ValueRef::Text(text) => match text {
                b"Fts" => Ok(SearchStrategy::Fts),
                b"Semantic" => Ok(SearchStrategy::Semantic),
                b"ReciprocalRankFusion" => Ok(SearchStrategy::ReciprocalRankFusion),
                _ => Err(FromSqlError::InvalidType),
            },
            _ => Err(FromSqlError::InvalidType),
        }
    }
}

#[derive(Debug, Error)]
enum SearchStrategyError {
    #[error(
        "'{0}' No es una estrategia de búsqueda soportada, usa 'fts', 'semantic_search' o 'rrf'"
    )]
    UnsupportedSearchStrategy(String),
}

fn build_conditions(
    constraints: Option<&HashMap<String, Vec<Constraint>>>,
) -> (Vec<String>, Vec<&dyn ToSql>) {
    let mut conditions = Vec::new();
    let mut binding_values: Vec<&dyn ToSql> = Vec::new();

    if let Some(constraints) = constraints {
        for (k, values) in constraints {
            for (i, cons) in values.iter().enumerate() {
                let param_name = format!(":{k}_{i}");
                let condition = match cons {
                    Constraint::Exact(_) => {
                        format!("LOWER({k}) like LOWER('%' || {param_name} || '%')")
                    }
                    Constraint::GreaterThan(_) => format!("{k} > {param_name}"),
                    Constraint::LesserThan(_) => format!("{k} < {param_name}"),
                };

                conditions.push(condition);

                let value = match cons {
                    Constraint::Exact(v)
                    | Constraint::GreaterThan(v)
                    | Constraint::LesserThan(v) => v,
                };

                binding_values.push(value);
            }
        }
    }

    (conditions, binding_values)
}

fn sqlite_value_to_string(row: &Row<'_>, idx: usize) -> Result<String, rusqlite::Error> {
    let val = match row.get_ref(idx)? {
        ValueRef::Text(text) => String::from_utf8_lossy(text).into_owned(),
        ValueRef::Real(real) => format!("{:.3}", -1. * real),
        ValueRef::Integer(int) => int.to_string(),
        _ => "Tipo de dato desconocido".to_owned(),
    };
    Ok(val)
}
