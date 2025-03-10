pub mod extractors;
pub mod routes;
pub mod search;
pub mod startup;
pub mod views;

use include_dir::{Dir, include_dir};
use rusqlite::types::FromSql;
use rusqlite::types::FromSqlError;
use rusqlite::types::FromSqlResult;
use rusqlite::types::ValueRef;
use rusqlite::{ToSql, types::ToSqlOutput};
use search::SearchStrategy;
use serde::Deserialize;
use serde::Serialize;
use std::fmt::{Display, Formatter};
use std::net::IpAddr;
use views::HistorialView;

pub static ASSETS: Dir = include_dir!("$CARGO_MANIFEST_DIR/ui/dist");

#[derive(Debug, Clone)]
pub struct ApplicationSettings {
    pub name: String,
    pub version: String,
    pub port: u16,
    pub host: IpAddr,
    pub open: bool,
}

impl ApplicationSettings {
    #[must_use]
    pub fn new(name: String, version: String, port: u16, host: IpAddr, open: bool) -> Self {
        Self {
            name,
            version,
            port,
            host,
            open,
        }
    }
}

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
        write!(f, "{}", content)
    }
}
