use crate::into_http::{HttpError, IntoHttp, SearchResult};
use axum::Json;
use eyre::Report;
use gulfi_openai::embed_single;
use gulfi_query::{
    Constraint::{Exact, GreaterThan, LesserThan},
    Query,
};

use reqwest::Client;
use rusqlite::{
    Connection, ToSql, params,
    types::{FromSql, FromSqlError, ToSqlOutput, ValueRef},
};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{debug, info};
use zerocopy::IntoBytes;

use crate::{startup::AppState, views::TableView};

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
        let db = Connection::open(app_state.db_path.clone())
            .expect("Deberia ser un path valido a una base de datos sqlite.");

        let document = app_state
            .documents
            .iter()
            .find(|x| x.name.to_lowercase() == params.document.to_lowercase())
            .unwrap();

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
                for (k, _) in constraints.iter() {
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

        // FIX: Lowercase and Uppercase words matter. They shouldn't.
        let (column_names, table, total_query_count) = match self {
            SearchStrategy::Fts => {
                let search = {
                    let start = "select rank as score,";
                    let mut rest = String::new();

                    for field in &document.fields {
                        if !field.vec_input {
                            rest.push_str(&format!("{},", field.name));
                        }
                    }

                    let doc_name = document.name.clone();
                    format!(
                        "{start}{rest}  highlight(fts_{doc_name}, 0, '<b style=\"color: green;\">', '</b>') as input, 'fts' as match_type from fts_{doc_name}",
                    )
                };

                let mut conditions = Vec::new();
                let mut binding_values: Vec<&dyn ToSql> = Vec::new();

                if let Some(contraints) = &query.constraints {
                    for (k, values) in contraints {
                        for (i, cons) in values.iter().enumerate() {
                            let param_name = format!(":{}_{i}", k);
                            let condition = match cons {
                                Exact(_) => format!("LOWER({k}) = LOWER({param_name})"),
                                GreaterThan(_) => format!("{k} > {param_name}"),
                                LesserThan(_) => format!("{k} < {param_name}"),
                            };

                            let value = match cons {
                                Exact(val) => val,
                                GreaterThan(val) => val,
                                LesserThan(val) => val,
                            };

                            conditions.push(condition);
                            binding_values.push(value);
                        }
                    }
                }

                conditions.push("vec_input match '\"' || :query || '\"' ".to_owned());
                binding_values.push(&query.query as &dyn ToSql);

                let where_clause = if conditions.is_empty() {
                    String::new()
                } else {
                    format!("where {}", conditions.join(" and "))
                };

                let sql = format!("{search} {where_clause}");

                // dbg!("{:#?}", &sql);

                let mut stmt = db.prepare(&sql)?;
                let column_names: Vec<String> =
                    stmt.column_names().into_iter().map(String::from).collect();

                let table = stmt
                    .query_map(&*binding_values, |row| {
                        let mut data = Vec::new();

                        for i in 0..row.as_ref().column_count() {
                            let val = match row.get_ref(i)? {
                                ValueRef::Text(text) => String::from_utf8_lossy(text).into_owned(),
                                ValueRef::Real(real) => format!("{:.3}", -1. * real),
                                ValueRef::Integer(int) => int.to_string(),
                                _ => "Tipo de dato desconocido".to_owned(),
                            };
                            data.push(val);
                        }

                        Ok(data)
                    })?
                    .collect::<Result<Vec<Vec<String>>, _>>()?;

                let count = table.len();

                (column_names, table, count)
            }
            SearchStrategy::Semantic => {
                let query_emb = embed_single(query.query.to_owned(), client)
                    .await
                    .map_err(|err| tracing::error!("{err}"))
                    .expect("Fallo al crear un embedding del query");

                let embedding = query_emb.as_bytes();

                let search = {
                    let start = format!("select vec_{}.distance,", document.name);
                    let mut rest = String::new();

                    for field in &document.fields {
                        if !field.vec_input {
                            rest.push_str(&format!("{}.{},", document.name, field.name));
                        }
                    }

                    let doc_name = document.name.clone();
                    format!(
                        "{start} {rest} {doc_name}.vec_input as input, 'vec' as match_type from vec_{doc_name} left join {doc_name} on {doc_name}.id = vec_{doc_name}.row_id"
                    )
                };

                let mut conditions = Vec::new();

                let mut binding_values: Vec<&dyn ToSql> = Vec::new();

                if let Some(contraints) = &query.constraints {
                    for (k, values) in contraints {
                        for (i, cons) in values.iter().enumerate() {
                            let param_name = format!(":{}_{i}", k);
                            let condition = match cons {
                                Exact(_) => format!("LOWER({k}) = LOWER({param_name})"),
                                GreaterThan(_) => format!("{k} > {param_name}"),
                                LesserThan(_) => format!("{k} < {param_name}"),
                            };

                            let value = match cons {
                                Exact(val) => val,
                                GreaterThan(val) => val,
                                LesserThan(val) => val,
                            };

                            conditions.push(condition);
                            binding_values.push(value);
                        }
                    }
                }

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

                let mut stmt = db.prepare(&sql)?;
                let column_names: Vec<String> =
                    stmt.column_names().into_iter().map(String::from).collect();

                let table = stmt
                    .query_map(&*binding_values, |row| {
                        let mut data = Vec::new();

                        for i in 0..row.as_ref().column_count() {
                            let val = match row.get_ref(i)? {
                                ValueRef::Text(text) => String::from_utf8_lossy(text).into_owned(),
                                ValueRef::Real(real) => format!("{:.3}", -1. * real),
                                ValueRef::Integer(int) => int.to_string(),
                                _ => "Tipo de dato desconocido".to_owned(),
                            };
                            data.push(val);
                        }

                        Ok(data)
                    })?
                    .collect::<Result<Vec<Vec<String>>, _>>()?;

                let count = table.len();

                (column_names, table, count)
            }
            SearchStrategy::ReciprocalRankFusion => {
                let query_emb = embed_single(query.query.to_owned(), client)
                    .await
                    .map_err(|err| tracing::error!("{err}"))
                    .expect("Fallo al crear un embedding del query");

                let embedding = query_emb.as_bytes();

                let build_final_query = |conditions: &str| -> String {
                    let doc_name = document.name.clone();
                    let mut fields = String::new();

                    for field in &document.fields {
                        if !field.vec_input {
                            fields.push_str(&format!("{doc_name}.{},", field.name));
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
                            let param_name = format!(":{}_{i}", k);
                            let condition = match cons {
                                Exact(_) => format!("LOWER({k}) = LOWER({param_name})"),
                                GreaterThan(_) => format!("{k} > {param_name}"),
                                LesserThan(_) => format!("{k} < {param_name}"),
                            };

                            let value = match cons {
                                Exact(val) => val,
                                GreaterThan(val) => val,
                                LesserThan(val) => val,
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

                // dbg!("{:#?}", &sql);
                // dbg!("{:#?}", &binding_values.len());

                let mut stmt = db.prepare(&sql)?;
                let column_names: Vec<String> =
                    stmt.column_names().into_iter().map(String::from).collect();

                let table = stmt
                    .query_map(&*binding_values, |row| {
                        let mut data = Vec::new();

                        for i in 0..row.as_ref().column_count() {
                            let val = match row.get_ref(i)? {
                                ValueRef::Text(text) => String::from_utf8_lossy(text).into_owned(),
                                ValueRef::Real(real) => format!("{:.3}", -1. * real),
                                ValueRef::Integer(int) => int.to_string(),
                                _ => "Tipo de dato desconocido".to_owned(),
                            };
                            data.push(val);
                        }

                        Ok(data)
                    })?
                    .collect::<Result<Vec<Vec<String>>, _>>()?;

                let count = table.len();

                (column_names, table, count)
            }
        };

        info!(
            "Busqueda para el query: `{}`, exitosa! {} registros",
            query.query, total_query_count,
        );

        let table = TableView {
            msg: format!("Hay un total de {} resultados.", total_query_count),
            columns: column_names,
            rows: table,
        };

        update_historial(&db, &params, document.name.clone())?;

        Json(table).into_http()
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

fn update_historial(db: &Connection, values: &SearchParams, doc: String) -> Result<(), HttpError> {
    let updated = db.execute(
        "insert or replace into historial(query, strategy, doc, peso_fts, peso_semantic, neighbors) values (?,?,?,?,?,?)",
        params![values.search_str, values.strategy,doc, values.peso_fts, values.peso_semantic, values.k_neighbors],
    )?;
    info!("{} registros fueron añadidos al historial!", updated);

    Ok(())
}
