mod assets;
mod serve_ui;
pub use serve_ui::*;
mod favoritos;
pub use favoritos::*;
pub use health_check::*;
mod health_check;
mod historial;
pub use historial::*;
mod index;
pub(crate) mod search;
pub use search::*;

use serde_json::json;

use axum::{Json, async_trait, extract::FromRequestParts};
use color_eyre::Report;
use gulfi_openai::embed_single;
use http::{Uri, request::Parts};

use gulfi_common::{HttpError, IntoHttp, SearchResult, SearchString};
use gulfi_sqlite::SearchQueryBuilder;
use gulfi_ui::{Sexo, Table};
use reqwest::Client;
use rusqlite::Connection;

use serde::{Deserialize, de::DeserializeOwned};
use thiserror::Error;
use tracing::{debug, error, info};
use zerocopy::IntoBytes;

#[derive(Deserialize, Debug, Clone)]
pub struct Params {
    #[serde(rename = "query")]
    search_str: String,
    // doc: String,
    strategy: SearchStrategy,
    sexo: Sexo,
    edad_min: u64,
    edad_max: u64,
    peso_fts: f32,
    peso_semantic: f32,
    #[serde(rename = "k")]
    k_neighbors: u64,
}

#[derive(Deserialize, Debug, Clone, Copy)]
pub enum SearchStrategy {
    Fts,
    Semantic,
    ReciprocalRankFusion,
    // KeywordFirst,
    // ReRankBySemantics,
}

impl SearchStrategy {
    pub async fn search(self, db_path: &str, client: &Client, params: Params) -> SearchResult {
        let db = Connection::open(db_path)
            .expect("Deberia ser un path valido a una base de datos sqlite.");

        let search = SearchString::parse(&params.search_str);

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
                let search = &"select
                    rank as score,
                    email,
                    provincia,
                    ciudad,
                    edad,
                    sexo,
                    highlight(fts_tnea, 0, '<b style=\"color: green;\">', '</b>') as input,
                    'fts' as match_type
                    from fts_tnea
                    where vec_input match '\"' || :query || '\"'
                    "
                .to_owned();

                let mut search_query = SearchQueryBuilder::new(&db, search);

                search_query.add_bindings(&[&query]);

                search_query.add_statement(
                    " and cast(edad as integer) between :edad_min and :edad_max ",
                    &[&params.edad_min, &params.edad_max],
                );

                if provincia.is_some() {
                    search_query.add_statement(" and provincia like :provincia", &[&provincia]);
                }
                if ciudad.is_some() {
                    search_query.add_statement(" and ciudad like :ciudad", &[&ciudad]);
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

                search_query.add_statement_str(" order by rank");

                let (c, t, tqc) = search_query.build().execute()?;

                (c, t, tqc)
            }
            SearchStrategy::Semantic => {
                let start = std::time::Instant::now();
                let query_emb = match db.query_row(
                    "select embedding from historial where query = ?",
                    [&params.search_str],
                    |row| row.get(0),
                ) {
                    Err(_) => {
                        info!("Creando nuevo embedding!");
                        let embedding = embed_single(query.clone(), client)
                            .await
                            .map_err(|err| tracing::error!("{err}"))
                            .expect("Fallo al crear un embedding del query")
                            .as_bytes()
                            .to_owned();

                        embedding
                    }
                    Ok(embedding) => {
                        info!("Re-utilizando el embedding!");
                        embedding
                    }
                };

                println!(
                    "Tiempo hasta que tengo el embedding {} ms",
                    start.elapsed().as_millis()
                );

                let start = std::time::Instant::now();
                let embedding = query_emb.as_bytes();

                println!(
                    "Tiempo hasta que actualice la db {} ms",
                    start.elapsed().as_millis()
                );

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

                let start = std::time::Instant::now();
                let (c, t, tqc) = search_query.build().execute()?;

                println!(
                    "Tiempo hasta que devuelo los datos {} ms",
                    start.elapsed().as_millis()
                );

                (c, t, tqc)
            }
            SearchStrategy::ReciprocalRankFusion => {
                let query_emb = match db.query_row(
                    "select embedding from historial where query = ?",
                    [&params.search_str],
                    |row| row.get(0),
                ) {
                    Err(_) => {
                        info!("Creando nuevo embedding!");
                        let embedding = embed_single(query.clone(), client)
                            .await
                            .map_err(|err| tracing::error!("{err}"))
                            .expect("Fallo al crear un embedding del query")
                            .as_bytes()
                            .to_owned();

                        embedding
                    }
                    Ok(embedding) => {
                        info!("Re-utilizando el embedding!");
                        embedding
                    }
                };

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

        let table = Table {
            msg: format!("Hay un total de {} resultados.", total_query_count),
            columns: column_names,
            rows: table,
        };

        gulfi_sqlite::update_historial(&db, &params.search_str)?;

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

#[derive(Debug, Error)]
enum SearchStrategyError {
    #[error(
        "'{0}' No es una estrategia de búsqueda soportada, usa 'fts', 'semantic_search', 'HKF' o 'rrf'"
    )]
    UnsupportedSearchStrategy(String),
}

pub struct SearchExtractor<T>(pub T);

impl<T> SearchExtractor<T>
where
    T: DeserializeOwned,
{
    pub fn try_from_uri(value: &Uri) -> Result<Self, HttpError> {
        let query = value.query().unwrap_or_default();
        let params = serde_urlencoded::from_str(query)?;
        Ok(SearchExtractor(params))
    }
}

#[async_trait]
impl<T, S> FromRequestParts<S> for SearchExtractor<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = HttpError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        Self::try_from_uri(&parts.uri)
    }
}
