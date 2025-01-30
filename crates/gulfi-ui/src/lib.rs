use std::fmt::Display;

use chrono::NaiveDateTime;
use rinja::Template;
use rusqlite::{
    ToSql,
    types::{FromSql, FromSqlError, ValueRef},
};
use serde::{Deserialize, Serialize};

pub static STYLES_CSS: &str = include_str!("../dist/styles.min.css");
pub static MAIN_JS: &str = include_str!("../dist/main.min.js");

#[derive(Template)]
#[template(path = "index.html")]
pub struct Index;

#[derive(Template)]
#[template(path = "table.html")]
pub struct Table {
    pub msg: String,
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

impl Default for Table {
    fn default() -> Self {
        Self {
            msg: "No se encontraron ningun registro.".to_owned(),
            columns: vec![],
            rows: vec![],
        }
    }
}

// El dataset solamente distingue entre estos dos.
#[derive(Deserialize, Debug, Clone, Default, PartialEq)]
pub enum Sexo {
    #[default]
    U,
    F,
    M,
}

impl ToSql for Sexo {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        let value = match self {
            Sexo::F => "F",
            Sexo::M => "M",
            Sexo::U => "U",
        };
        Ok(rusqlite::types::ToSqlOutput::from(value))
    }
}

impl FromSql for Sexo {
    fn column_result(value: ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        match value {
            ValueRef::Text(text) => match text {
                b"F" => Ok(Sexo::F),
                b"M" => Ok(Sexo::M),
                _ => Ok(Sexo::U),
            },
            _ => Err(FromSqlError::InvalidType),
        }
    }
}

impl Display for Sexo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let content = match self {
            Sexo::U => "No definido",
            Sexo::F => "F",
            Sexo::M => "M",
        };
        write!(f, "{}", content)
    }
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct Historial {
    pub id: u64,
    pub query: String,
}

impl Historial {
    #[must_use]
    pub fn new(id: u64, query: String) -> Self {
        Self { id, query }
    }
}

#[derive(Template, Debug, Clone, Default, Serialize)]
#[template(path = "favoritos.html")]
pub struct Favoritos {
    pub favoritos: Vec<Resultados>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct Resultados {
    pub id: u64,
    pub nombre: String,
    pub data: String,
    pub fecha: String,
    pub busquedas: Vec<String>,
}

impl Resultados {
    #[must_use]
    pub fn new(
        id: u64,
        nombre: String,
        data: String,
        fecha: NaiveDateTime,
        busquedas: Vec<String>,
    ) -> Self {
        Self {
            id,
            nombre,
            data,
            fecha: fecha.format("%b %d, %Y").to_string(),
            busquedas,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // #[test]
    // fn it_works() {
    //     let result = add(2, 2);
    //     assert_eq!(result, 4);
    // }
}
