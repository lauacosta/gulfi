use crate::SearchStrategy;
use chrono::NaiveDateTime;
use serde::Serialize;

#[derive(Serialize, Debug, Clone)]
pub struct HistorialParams {
    filters: Option<String>,
    strategy: SearchStrategy,
    peso_fts: f32,
    peso_semantic: f32,
    neighbors: u64,
    fecha: String,
}

impl HistorialParams {
    #[must_use]
    pub fn new(
        filters: Option<String>,
        strategy: SearchStrategy,
        peso_fts: f32,
        peso_semantic: f32,
        neighbors: u64,
        fecha: NaiveDateTime,
    ) -> Self {
        Self {
            filters,
            strategy,
            peso_fts,
            peso_semantic,
            neighbors,
            fecha: fecha.format("%b %d, %Y %H:%M").to_string(),
        }
    }
}

#[derive(Serialize, Debug, Clone)]
pub struct HistorialFullView {
    id: u64,
    query: String,
    #[serde(flatten)]
    params: HistorialParams,
}

impl HistorialFullView {
    #[must_use]
    pub fn new(id: u64, query: String, params: HistorialParams) -> Self {
        Self { id, query, params }
    }
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct HistorialView {
    pub id: u64,
    pub query: String,
}

impl HistorialView {
    #[must_use]
    pub fn new(id: u64, query: String) -> Self {
        Self { id, query }
    }
}

#[derive(Serialize)]
pub struct TableView {
    pub msg: String,
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
}
