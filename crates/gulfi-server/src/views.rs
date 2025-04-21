use crate::SearchStrategy;
use chrono::NaiveDateTime;
use serde::Serialize;

#[derive(Serialize, Debug, Clone)]
pub struct HistorialFullView {
    id: u64,
    query: String,
    filters: Option<String>,
    strategy: SearchStrategy,
    peso_fts: f32,
    peso_semantic: f32,
    neighbors: u64,
    fecha: String,
}

impl HistorialFullView {
    #[must_use]
    pub fn new(
        id: u64,
        query: String,
        filters: Option<String>,
        strategy: SearchStrategy,
        peso_fts: f32,
        peso_semantic: f32,
        neighbors: u64,
        fecha: NaiveDateTime,
    ) -> Self {
        Self {
            id,
            query,
            filters,
            strategy,
            peso_fts,
            peso_semantic,
            neighbors,
            fecha: fecha.format("%b %d, %Y %H:%M").to_string(),
        }
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

#[derive(Debug, Clone, Default, Serialize)]
pub struct FavoritosView {
    pub favoritos: Vec<ResultadosView>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct ResultadosView {
    pub id: u64,
    pub nombre: String,
    pub data: String,
    pub tipos: Vec<String>,
    pub fecha: String,
    pub busquedas: Vec<String>,
}

impl ResultadosView {
    #[must_use]
    pub fn new(
        id: u64,
        nombre: String,
        data: String,
        tipos: Vec<String>,
        fecha: NaiveDateTime,
        busquedas: Vec<String>,
    ) -> Self {
        Self {
            id,
            nombre,
            data,
            tipos,
            fecha: fecha.format("%b %d, %Y %H:%M").to_string(),
            busquedas,
        }
    }
}

#[derive(Serialize)]
pub struct TableView {
    pub msg: String,
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
}
