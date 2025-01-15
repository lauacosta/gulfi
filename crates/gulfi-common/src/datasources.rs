use eyre::eyre;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use tracing::info;

#[derive(Serialize, Deserialize, Debug)]
pub struct Source {
    pub name: String,
    pub fields: Vec<Field>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Field {
    pub name: String,
    pub template_member: bool,
}

#[derive(Debug, PartialEq)]
pub enum DataSources {
    Csv,
    Json,
}

impl DataSources {
    pub fn from_extension(ext: &str) -> eyre::Result<Self> {
        let file = match ext {
            "csv" => DataSources::Csv,
            "json" => DataSources::Json,
            _ => return Err(eyre!("Extension desconocida {ext}")),
        };

        Ok(file)
    }
}

pub fn parse_sources(path: impl AsRef<Path>) -> eyre::Result<Vec<(PathBuf, DataSources)>> {
    let mut datasources = Vec::new();

    info!("Escaneando los archivos disponibles...");
    for file in std::fs::read_dir(&path)? {
        let path = file?.path().clone();

        if path.is_file() {
            if let Some(ext) = path.extension().and_then(|ext| ext.to_str()) {
                let file = DataSources::from_extension(ext)?;
                datasources.push((path, file));
            }
        }
    }

    info!("Escaneando los archivos disponibles... listo!");

    Ok(datasources)
}
