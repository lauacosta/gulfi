pub mod bg_tasks;
pub mod configuration;
pub mod extractors;
pub mod formatter;
pub mod into_http;
pub mod routes;
pub mod search;
pub mod startup;
pub mod telemetry;
pub mod views;

use include_dir::{Dir, include_dir};
use rusqlite::{
    ToSql,
    types::{FromSql, FromSqlError, FromSqlResult, ToSqlOutput, ValueRef},
};
use search::SearchStrategy;
use serde::Deserialize;
use serde::Serialize;
use std::fmt::{Display, Formatter};
use views::HistorialView;

pub static ASSETS: Dir = include_dir!("$CARGO_MANIFEST_DIR/ui/dist");

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
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
        write!(f, "{content}")
    }
}
