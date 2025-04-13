use axum::Json;
use eyre::Report;
use gulfi_common::{HttpError, IntoHttp, SearchResult, SearchString};
use gulfi_openai::embed_single;
use gulfi_query::{
    Constraint::{Exact, GreaterThan, LesserThan},
    Query,
};
use gulfi_sqlite::SearchQueryBuilder;
use reqwest::Client;
use rusqlite::{
    Connection, ToSql, params,
    types::{FromSql, FromSqlError, ToSqlOutput, ValueRef},
};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{debug, info};
use zerocopy::IntoBytes;

use crate::{Sexo, startup::AppState, views::TableView};

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default)]
pub enum SearchStrategy {
    #[default]
    Fts,
    Semantic,
    ReciprocalRankFusion,
    // KeywordFirst,
    // ReRankBySemantics,
}

#[derive(Deserialize, Debug, Clone)]
pub struct SearchParams {
    #[serde(rename = "query")]
    pub search_str: String,
    pub document: String,
    pub strategy: SearchStrategy,
    pub sexo: Sexo,
    pub edad_min: u64,
    pub edad_max: u64,
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

        let search = SearchString::parse(&params.search_str);

        let document = app_state
            .documents
            .iter()
            .find(|x| x.name.to_lowercase() == params.document.to_lowercase())
            .unwrap();

        debug!(?search);

        let query = search.query.trim().to_owned();
        let provincia = search.provincia;
        let ciudad = search.ciudad;

        let weight_vec = params.peso_semantic / 100.0;
        let weight_fts: f32 = params.peso_fts / 100.0;
        let rrf_k: i64 = 60;
        let k = params.k_neighbors;

        // FIX: Odio tener que usar el QueryBuilder
        let (column_names, table, total_query_count) = match self {
            SearchStrategy::Fts => {
                let string = format!("query:{}", params.search_str);
                let query = Query::parse(&string).unwrap();

                let search = {
                    let start = "select rank as score,";
                    let mut rest = String::new();

                    for field in &document.fields {
                        if !field.vec_input {
                            rest.push_str(&format!("{},", field.name));
                        }
                    }

                    format!(
                        "{start}{rest}  highlight(fts_tnea, 0, '<b style=\"color: green;\">', '</b>') as input, 'fts' as match_type from fts_{}",
                        document.name
                    )
                };

                // dbg!("{:?}", &search);

                let mut conditions = Vec::new();
                let mut bindings: Vec<(String, String)> = Vec::new();

                if let Some(contraints) = &query.constraints {
                    for (k, values) in contraints {
                        for (i, cons) in values.iter().enumerate() {
                            let param_name = format!(":{}_{i}", k);
                            let condition = match cons {
                                Exact(_) => format!("{k} = {param_name}"),
                                GreaterThan(_) => format!("{k} > {param_name}"),
                                LesserThan(_) => format!("{k} < {param_name}"),
                            };

                            let value = match cons {
                                Exact(val) => val,
                                GreaterThan(val) => val,
                                LesserThan(val) => val,
                            };

                            conditions.push(condition);
                            bindings.push((param_name.clone(), value.clone()));
                        }
                    }
                }

                let query_param = " :query ";
                conditions.push(format!("vec_input match '\"' || {query_param} || '\"' "));
                bindings.push((query_param.to_owned(), query.query.to_owned()));

                let where_clause = if conditions.is_empty() {
                    String::new()
                } else {
                    format!("where {}", conditions.join(" and "))
                };

                let sql = format!("{search} {where_clause}");

                dbg!("{:#?}", &sql);

                let mut stmt = db.prepare(&sql)?;

                let binding_values: Vec<&dyn ToSql> =
                    bindings.iter().map(|(_, val)| val as &dyn ToSql).collect();

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
                let query_emb = embed_single(query.clone(), client)
                    .await
                    .map_err(|err| tracing::error!("{err}"))
                    .expect("Fallo al crear un embedding del query");

                let embedding = query_emb.as_bytes();

                let search = &"
                select
                    vec_tnea.distance,
                    tnea.email,
                    tnea.provincia,
                    tnea.ciudad,
                    tnea.edad,
                    tnea.sexo,
                    tnea.vec_input,
                    'vec' as match_type
                from vec_tnea
                left join tnea on tnea.id = vec_tnea.row_id
                where vec_input_embedding match :embedding
                and k = 1000
                "
                .to_owned();

                let mut search_query = SearchQueryBuilder::new(&db, search);

                search_query.add_bindings(&[&embedding]);

                search_query.add_statement(
                    " and cast(edad as integer) between :edad_min and :edad_max ",
                    &[&params.edad_min, &params.edad_max],
                );

                if provincia.is_some() {
                    search_query
                        .add_statement(" and tnea.provincia like :provincia", &[&provincia]);
                }

                if ciudad.is_some() {
                    search_query.add_statement(" and tnea.ciudad like :ciudad", &[&ciudad]);
                }

                match params.sexo {
                    Sexo::M => {
                        search_query.add_statement(" and sexo = :sexo", &[&params.sexo]);
                    }
                    Sexo::F => {
                        search_query.add_statement(" and sexo = :sexo", &[&params.sexo]);
                    }
                    Sexo::U => (),
                };

                let (c, t, tqc) = search_query.build().execute()?;

                (c, t, tqc)
            }
            SearchStrategy::ReciprocalRankFusion => {
                let query_emb = embed_single(query.clone(), client)
                    .await
                    .map_err(|err| tracing::error!("{err}"))
                    .expect("Fallo al crear un embedding del query");

                let embedding = query_emb.as_bytes();

                let search = &"
            with vec_matches as (
                select
                    row_id,
                    row_number() over (order by distance) as rank_number,
                    distance
                from vec_tnea
                where
                    vec_input_embedding match :embedding
                    and k = :k
                ),

            fts_matches as (
                select
                    rowid as row_id,
                    row_number() over (order by rank) as rank_number,
                    rank as score
                from fts_tnea
                where vec_input match '\"' || :query || '\"'
                ),

            final as (
                select
                    tnea.email,
                    tnea.edad,
                    tnea.sexo,
                    tnea.provincia, 
                    tnea.ciudad,
                    tnea.vec_input,
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
                join tnea on tnea.id = coalesce(fts_matches.row_id, vec_matches.row_id)
                where cast(tnea.edad as integer) between :edad_min and :edad_max
            "
                .to_owned();

                let mut search_query = SearchQueryBuilder::new(&db, search);

                search_query.add_bindings(&[
                    &embedding,
                    &k,
                    &query,
                    &rrf_k,
                    &weight_fts,
                    &weight_vec,
                    &params.edad_min,
                    &params.edad_max,
                ]);

                if provincia.is_some() {
                    search_query
                        .add_statement(" and tnea.provincia like :provincia", &[&provincia]);
                }

                if ciudad.is_some() {
                    search_query.add_statement(" and tnea.ciudad like :ciudad", &[&ciudad]);
                }

                match params.sexo {
                    Sexo::M => {
                        search_query.add_statement(" and sexo = :sexo", &[&params.sexo]);
                    }
                    Sexo::F => {
                        search_query.add_statement(" and sexo = :sexo", &[&params.sexo]);
                    }
                    Sexo::U => (),
                };

                search_query.add_statement_str(" order by combined_rank desc ");

                search_query.add_statement_str(
                    ") 
                    select * from final;",
                );

                let (c, t, tqc) = search_query.build().execute()?;
                (c, t, tqc)
            }
        };

        info!(
            "Busqueda para el query: `{}`, exitosa! {} registros",
            query, total_query_count,
        );

        let table = TableView {
            msg: format!("Hay un total de {} resultados.", total_query_count),
            columns: column_names,
            rows: table,
        };

        update_historial(&db, &params)?;

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

fn update_historial(db: &Connection, values: &SearchParams) -> Result<(), HttpError> {
    let updated = db.execute(
        "insert or replace into historial(query, strategy, sexo, edad_min, edad_max, peso_fts, peso_semantic, neighbors) values (?,?,?,?,?,?,?,?)",
        params![values.search_str, values.strategy, values.sexo, values.edad_min, values.edad_max, values.peso_fts, values.peso_semantic, values.k_neighbors],
    )?;
    info!("{} registros fueron añadidos al historial!", updated);

    Ok(())
}
