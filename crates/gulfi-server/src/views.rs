use chrono::NaiveDateTime;
use serde::Serialize;

use crate::{SearchStrategy, Sexo};

#[derive(Serialize, Debug, Clone)]
pub struct HistorialFullView {
    id: u64,
    query: String,
    strategy: SearchStrategy,
    sexo: Sexo,
    edad_min: u64,
    edad_max: u64,
    peso_fts: f32,
    peso_semantic: f32,
    neighbors: u64,
}

impl HistorialFullView {
    #[must_use]
    pub fn new(
        id: u64,
        query: String,
        strategy: SearchStrategy,
        sexo: Sexo,
        edad_min: u64,
        edad_max: u64,
        peso_fts: f32,
        peso_semantic: f32,
        neighbors: u64,
    ) -> Self {
        Self {
            id,
            query,
            strategy,
            sexo,
            edad_min,
            edad_max,
            peso_fts,
            peso_semantic,
            neighbors,
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

impl Default for TableView {
    fn default() -> Self {
        Self {
            msg: "No se encontraron ningun registro.".to_owned(),
            columns: vec![],
            rows: vec![],
        }
    }
}
