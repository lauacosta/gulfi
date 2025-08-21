use crate::into_http::HttpError;

use axum::response::{Sse, sse::Event};
use eyre::Report;
use futures::Stream;
use gulfi_ingest::Document;
use gulfi_query::{
    Constraint::{self},
    Query,
};
use std::{collections::BTreeMap, convert::Infallible, fmt::Write, sync::Arc};

use reqwest::Client;
use rusqlite::{
    ToSql,
    types::{FromSql, FromSqlError, ToSqlOutput, ValueRef},
};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{error, info_span, instrument};
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
    pub batch_size: Option<usize>,
}

impl SearchStrategy {
    #[instrument(name = "searching", skip(self, state, client, params), fields(source = tracing::field::Empty))]
    pub async fn search_stream(
        self,
        state: ServerState,
        client: Client,
        params: SearchParams,
    ) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
        let state = state.clone();
        let client = client.clone();

        let s = async_stream::stream! {
            let search_result = match Self::prepare_search(&state, &params).await {
                Ok(res) => res,
                Err(e) => {
                    let error_msg = StreamMessage::Error { msg: e.to_string() };
                    yield Ok(Event::default().data(serde_json::to_string(&error_msg).unwrap()));
                    return;
                }
            };

            let columns: Vec<String> = search_result
                .document
                .fields
                .iter()
                .map(|f| f.name.clone())
                .collect();
            let metadata = StreamMessage::Metadata { columns };
            yield Ok(Event::default().data(serde_json::to_string(&metadata).unwrap()));


        let mut row_count = 0;
        match SearchStrategy::stream_results(
            search_result,
            client,
            state,
            params.batch_size.unwrap_or(10)
        )
        .await
        {
            Ok(mut result_stream) => {
                 while let Some(result) = result_stream.recv().await {
                    match result {
                        Ok(msg) => {
                            match &msg {
                                StreamMessage::Rows { data } => row_count += data.len(),
                                _ => {}
                            }
                            yield Ok(Event::default().data(serde_json::to_string(&msg).unwrap()));
                        }
                        Err(err) => {
                            let error_msg = StreamMessage::Error { msg: err.to_string() };
                            yield Ok(Event::default().data(serde_json::to_string(&error_msg).unwrap()));
                            break;
                        }
                    }
                }
            }
            Err(err) => {
                let error_msg = StreamMessage::Error { msg: err.to_string() };
                yield Ok(Event::default().data(serde_json::to_string(&error_msg).unwrap()));

            }
        }
            let complete = StreamMessage::Complete { total_sent: row_count };
            yield Ok(Event::default().data(serde_json::to_string(&complete).unwrap()));
        };

        Sse::new(s)
    }

    async fn prepare_search(
        state: &ServerState,
        params: &SearchParams,
    ) -> Result<StreamSearch, HttpError> {
        let document = state
            .documents
            .iter()
            .find(|x| x.name.eq_ignore_ascii_case(&params.document))
            .ok_or_else(|| {
                HttpError::missing_document(format!("Document {} not found", params.document))
            })?;

        let query =
            Query::parse(&format!("query: {}", params.search_str)).map_err(HttpError::from)?;

        validate_query_constraints(document, &query)?;

        Ok(StreamSearch {
            document: document.clone(),
            query,
            strategy: params.strategy,
            k_neighbors: params.k_neighbors,
            weight_fts: params.peso_fts,
            weight_vec: params.peso_semantic,
        })
    }

    async fn stream_results(
        search: StreamSearch,
        client: Client,
        state: ServerState,
        batch_size: usize,
    ) -> eyre::Result<tokio::sync::mpsc::Receiver<Result<StreamMessage, eyre::Error>>> {
        let pool = state.pool.clone();
        let conn_handle = {
            let span = info_span!("conn.acquire");
            let _guard = span.enter();
            pool.acquire().await?
        };

        let query_emb = {
            let span = info_span!("query.embedding");
            let _guard = span.enter();

            state
                .get_embeddings(&search.query.query, &client, search.strategy, &span)
                .await?
                .into_inner()
        };

        let (result_tx, result_rx) =
            tokio::sync::mpsc::channel::<Result<StreamMessage, eyre::Error>>(batch_size * 2);
        let (tx, mut rx) =
            tokio::sync::mpsc::channel::<Result<Vec<String>, rusqlite::Error>>(batch_size * 2);

        // TODO: Refactor binding_refs into a simpler type as binding_values is at the moment just a Vec over boxed types
        tokio::task::spawn_blocking(move || -> eyre::Result<()> {
            let (sql, binding_values) = Self::build_query(&search, query_emb)?;
            let mut stmt = conn_handle.prepare_cached(&sql)?;

            let binding_refs: Vec<&dyn ToSql> =
                binding_values.iter().map(|b| &**b as &dyn ToSql).collect();

            let span = info_span!("search.query");
            let _guard = span.enter();

            let rows = stmt.query_map(&*binding_refs, process_row_to_strings)?;

            for row_result in rows {
                if tx.blocking_send(row_result).is_err() {
                    break; // Receiver dropped
                }
            }
            Ok(())
        });

        tokio::spawn(async move {
            let mut batch = Vec::with_capacity(batch_size);

            while let Some(row_result) = rx.recv().await {
                match row_result {
                    Ok(row_data) => {
                        batch.push(row_data);

                        if batch.len() >= batch_size {
                            let batch_msg = StreamMessage::Rows {
                                data: std::mem::take(&mut batch),
                            };
                            if result_tx.send(Ok(batch_msg)).await.is_err() {
                                break; // Receiver dropped
                            }
                        }
                    }
                    Err(e) => {
                        error!("sqlite row error: {e:?}");
                        let _ = result_tx
                            .send(Err(eyre::eyre!("Database error: {}", e)))
                            .await;
                        break;
                    }
                }
            }
            if !batch.is_empty() {
                let batch_msg = StreamMessage::Rows {
                    data: std::mem::take(&mut batch),
                };
                let _ = result_tx.send(Ok(batch_msg)).await;
            }
        });

        Ok(result_rx)
    }

    fn build_query(
        search: &StreamSearch,
        query_emb: Option<Arc<Vec<f32>>>,
    ) -> Result<(String, Vec<Box<dyn ToSql + Send + Sync>>), HttpError> {
        let result = match search.strategy {
            SearchStrategy::Fts => Self::build_fts_query(search),
            SearchStrategy::Semantic => Self::build_semantic_query(search, query_emb)?,
            SearchStrategy::ReciprocalRankFusion => Self::build_rrf_query(search, query_emb)?,
        };
        Ok(result)
    }

    fn build_semantic_query(
        search: &StreamSearch,
        query_emb: Option<Arc<Vec<f32>>>,
    ) -> Result<(String, Vec<Box<dyn ToSql + Send + Sync>>), HttpError> {
        let embedding = query_emb.ok_or_else(|| HttpError::Internal {
            err: "failed to create embedding".to_owned(),
        })?;
        let embedding = embedding.as_bytes().to_vec();

        let search_str = {
            let start = format!("select vec_{}.distance,", search.document.name);
            let mut fields = String::new();

            for field in &search.document.fields {
                if !field.vec_input {
                    let _ = write!(fields, " {}.{},", search.document.name, field.name);
                }
            }

            let doc_name = search.document.name.clone();
            format!(
                "{start} {fields} {doc_name}.vec_input as input, 'vec' as match_type from vec_{doc_name} left join {doc_name} on {doc_name}.id = vec_{doc_name}.row_id"
            )
        };

        let (mut conditions, mut binding_values) =
            build_conditions_owned(search.query.constraints.as_ref());

        conditions.push("k = :k".to_owned());
        binding_values.push(Box::new(search.k_neighbors));

        conditions.push("vec_input_embedding match :embedding ".to_owned());
        binding_values.push(Box::new(embedding));

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("where {}", conditions.join(" and "))
        };

        let sql = format!("{search_str} {where_clause}");

        Ok((sql, binding_values))
    }

    fn build_fts_query(search: &StreamSearch) -> (String, Vec<Box<dyn ToSql + Send + Sync>>) {
        let search_str = {
            let start = "select rank as score,";
            let mut fields = String::new();

            for field in &search.document.fields {
                if !field.vec_input {
                    let _ = write!(fields, "{},", field.name);
                }
            }

            let doc_name = search.document.name.clone();
            format!(
                "{start}{fields}  highlight(fts_{doc_name}, 0, '<b style=\"color: green;\">', '</b>') as input, 'fts' as match_type from fts_{doc_name}",
            )
        };

        let (mut conditions, mut binding_values) =
            build_conditions_owned(search.query.constraints.as_ref());

        conditions.push("vec_input match '\"' || :query || '\"' ".to_owned());
        binding_values.push(Box::new(search.query.query.clone()));

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("where {}", conditions.join(" and "))
        };

        let sql = format!("{search_str} {where_clause}");

        (sql, binding_values)
    }

    fn build_rrf_query(
        search: &StreamSearch,
        query_emb: Option<Arc<Vec<f32>>>,
    ) -> Result<(String, Vec<Box<dyn ToSql + Send + Sync>>), HttpError> {
        let embedding = query_emb.ok_or_else(|| HttpError::Internal {
            err: "failed to create embedding".to_owned(),
        })?;
        let embedding = embedding.as_bytes().to_vec();

        let build_final_query = |conditions: &str| -> String {
            let doc_name = search.document.name.clone();
            let mut fields = String::new();

            for field in &search.document.fields {
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
        let mut binding_values: Vec<Box<dyn ToSql + Send + Sync>> = vec![
            Box::new(embedding),
            Box::new(search.k_neighbors),
            Box::new(search.query.query.clone()),
            // TODO:
            Box::new(60),
            Box::new(search.weight_fts),
            Box::new(search.weight_vec),
        ];

        let (a, b) = build_conditions_owned(search.query.constraints.as_ref());
        for x in a {
            conditions.push(x);
        }
        for x in b {
            binding_values.push(x);
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("where {}", conditions.join(" and "))
        };

        let sql = build_final_query(&where_clause);

        Ok((sql, binding_values))
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

fn build_conditions_owned(
    constraints: Option<&BTreeMap<String, Vec<Constraint>>>,
) -> (Vec<String>, Vec<Box<dyn ToSql + Send + Sync>>) {
    let mut conditions = Vec::new();
    let mut binding_values: Vec<Box<dyn ToSql + Send + Sync + '_>> = Vec::new();

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

                binding_values.push(Box::new(value.clone()));
            }
        }
    }

    (conditions, binding_values)
}

fn process_row_to_strings(row: &rusqlite::Row<'_>) -> Result<Vec<String>, rusqlite::Error> {
    (0..row.as_ref().column_count())
        .map(|idx| {
            let val = match row.get_ref(idx)? {
                ValueRef::Text(text) => String::from_utf8_lossy(text).into_owned(),
                ValueRef::Real(real) => format!("{:.3}", -real),
                ValueRef::Integer(int) => int.to_string(),
                _ => "Tipo de dato desconocido".to_owned(),
            };
            Ok(val)
        })
        .collect()
}

fn validate_query_constraints(document: &Document, query: &Query) -> Result<(), HttpError> {
    let valid_fields: Vec<String> = document
        .fields
        .iter()
        .filter(|field| !field.vec_input)
        .map(|field| field.name.clone())
        .collect();

    if let Some(constraints) = &query.constraints {
        let invalid_fields: Vec<String> = constraints
            .keys()
            .filter(|field| !valid_fields.contains(field))
            .cloned()
            .collect();

        if !invalid_fields.is_empty() {
            return Err(HttpError::bad_request(
                "You are looking for fields that don't exist in the document.".to_owned(),
                valid_fields,
                invalid_fields,
            ));
        }
    }

    Ok(())
}

struct StreamSearch {
    document: Document,
    query: Query,
    strategy: SearchStrategy,
    k_neighbors: u64,
    weight_fts: f32,
    weight_vec: f32,
}

#[derive(Serialize)]
#[serde(tag = "type")]
enum StreamMessage {
    #[serde(rename = "metadata")]
    Metadata {
        columns: Vec<String>,
        // total_estimated: Option<usize>,
    },
    #[serde(rename = "row")]
    _Row { data: Vec<String> },
    #[serde(rename = "rows")]
    Rows { data: Vec<Vec<String>> },
    #[serde(rename = "complete")]
    Complete { total_sent: usize },
    #[serde(rename = "error")]
    Error { msg: String },
}
