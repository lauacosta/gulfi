use std::fmt::{Display, Formatter};

use chrono::NaiveDateTime;
use include_dir::{Dir, include_dir};
use rusqlite::{
    ToSql,
    types::{FromSql, FromSqlError, FromSqlResult, ToSqlOutput, ValueRef},
};
use serde::{Deserialize, Serialize};

pub static ASSETS: Dir = include_dir!("$CARGO_MANIFEST_DIR/ui/dist");

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

#[derive(Debug, Clone, Default, Serialize)]
pub struct Favoritos {
    pub favoritos: Vec<Resultados>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct Resultados {
    pub id: u64,
    pub nombre: String,
    pub data: String,
    pub tipos: Vec<String>,
    pub fecha: String,
    pub busquedas: Vec<String>,
}

impl Resultados {
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
            fecha: fecha.format("%b %d, %Y").to_string(),
            busquedas,
        }
    }
}

#[derive(Serialize)]
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

#[derive(Deserialize, Debug, Clone, Default, PartialEq)]
pub enum Sexo {
    #[default]
    U,
    F,
    M,
}

impl ToSql for Sexo {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        let value = match self {
            Sexo::F => "F",
            Sexo::M => "M",
            Sexo::U => "U",
        };
        Ok(ToSqlOutput::from(value))
    }
}

impl FromSql for Sexo {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
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
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let content = match self {
            Sexo::U => "No definido",
            Sexo::F => "F",
            Sexo::M => "M",
        };
        write!(f, "{}", content)
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//
//     // #[test]
//     // fn it_works() {
//     //     let result = add(2, 2);
//     //     assert_eq!(result, 4);
//     // }
// }
