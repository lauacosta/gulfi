#![allow(clippy::too_many_lines)]
use crate::{
    into_http::{HttpError, IntoHttp, SearchResult},
    startup::WriteJob,
};
use axum::Json;
use eyre::Report;
use gulfi_openai::embed_single;
use gulfi_query::{
    Constraint::{self, Exact, GreaterThan, LesserThan},
    Query,
};
use std::{collections::HashMap, fmt::Write};

use reqwest::Client;
use rusqlite::{
    Row, ToSql,
    types::{FromSql, FromSqlError, ToSqlOutput, ValueRef},
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use thiserror::Error;
use tracing::{debug, info};
use zerocopy::IntoBytes;

use crate::startup::AppState;

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

impl SearchStrategy {
    pub async fn search(
        self,
        app_state: &AppState,
        client: &Client,
        params: SearchParams,
    ) -> SearchResult {
        let document = app_state
            .documents
            .iter()
            .find(|x| x.name.to_lowercase() == params.document.to_lowercase())
            .unwrap_or_else(|| panic!("No se ha encontrado el documento {}", params.document));

        let weight_vec = params.peso_semantic / 100.0;
        let weight_fts: f32 = params.peso_fts / 100.0;
        let rrf_k: i64 = 60;
        let k = params.k_neighbors;

        let string = format!("query:{}", params.search_str);
        let query = Query::parse(&string).map_err(HttpError::from)?;

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
            return Err(HttpError::BadRequest {
                message: "Buscas campos que no existen en tu documento.".to_owned(),
                valid_fields,
                invalid_fields,
            });
        }

        debug!(?query);

        let (column_names, table, total_query_count) = {
            let pool = app_state.connection_pool.clone();

            let conn_handle = pool.acquire().await?;

            debug!("{:?}", pool.stats());
            debug!("{:?}", pool.corruption_info());

            let res = match self {
                SearchStrategy::Fts => {
                    let search = {
                        let start = "select rank as score,";
                        let mut rest = String::new();

                        for field in &document.fields {
                            if !field.vec_input {
                                let _ = write!(rest, "{},", field.name);
                            }
                        }

                        let doc_name = document.name.clone();
                        format!(
                            "{start}{rest}  highlight(fts_{doc_name}, 0, '<b style=\"color: green;\">', '</b>') as input, 'fts' as match_type from fts_{doc_name}",
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

                    let mut stmt = conn_handle.prepare(&sql)?;

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
                SearchStrategy::Semantic => {
                    let query_emb = embed_single(query.query.clone(), client)
                        .await
                        .map_err(|err| tracing::error!("{err}"))
                        .expect("Fallo al crear un embedding del query");

                    let embedding = query_emb.as_bytes();

                    let search = {
                        let start = format!("select vec_{}.distance,", document.name);
                        let mut rest = String::new();

                        for field in &document.fields {
                            if !field.vec_input {
                                let _ = write!(rest, "{}.{}", document.name, field.name);
                            }
                        }

                        let doc_name = document.name.clone();
                        format!(
                            "{start} {rest} {doc_name}.vec_input as input, 'vec' as match_type from vec_{doc_name} left join {doc_name} on {doc_name}.id = vec_{doc_name}.row_id"
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

                    let mut stmt = conn_handle.prepare(&sql)?;
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
                    let query_emb = embed_single(query.query.clone(), client)
                        .await
                        .map_err(|err| tracing::error!("{err}"))
                        .expect("Fallo al crear un embedding del query");

                    let embedding = query_emb.as_bytes();

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
                                    Exact(_) => format!("LOWER({k}) = LOWER({param_name})"),
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

                    let mut stmt = conn_handle.prepare(&sql)?;
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
            };

            res
            // the conn_handle gets dropped
        };

        info!(
            "Busqueda para el query: `{}`, exitosa! {} registros",
            query.query, total_query_count,
        );

        if let Err(e) = app_state.writer.send(WriteJob::Historial {
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
                    Constraint::Exact(_) => format!("LOWER({k}) = LOWER({param_name})"),
                    Constraint::GreaterThan(_) => format!("{k} > {param_name}"),
                    Constraint::LesserThan(_) => format!("{k} < {param_name}"),
                };
                let value = match cons {
                    Constraint::Exact(v)
                    | Constraint::GreaterThan(v)
                    | Constraint::LesserThan(v) => v,
                };
                conditions.push(condition);
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
