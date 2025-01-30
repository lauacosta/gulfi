use std::path::{Path, PathBuf};

use camino::Utf8Path;
use eyre::eyre;
use serde::{Deserialize, Serialize};

use tracing::info;

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub struct Document {
    pub name: String,
    pub fields: Vec<Field>,
}

impl Document {
    pub fn generate_template(&self) -> String {
        let mut result = String::from("'  '");
        for i in &self.fields {
            if i.template_member {
                result.push_str(&format!(" || {} || '  '", i.name));
            }
        }

        result
    }
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

type A = Vec<(PathBuf, DataSources)>;
pub struct Folder {
    pub name: String,
    pub files: A,
}

pub fn parse_sources(path: impl AsRef<Path>, doc: &Document) -> eyre::Result<Folder> {
    let mut files = vec![];

    info!("Escaneando los archivos disponibles para {}...", doc.name);
    for entry in std::fs::read_dir(&path)? {
        let path = entry?.path();
        let utf_8_path = Utf8Path::from_path(&path).expect("Deberia ser UTF-8");

        if utf_8_path.is_dir() {
            let name = utf_8_path.file_stem().unwrap().to_string();
            if doc.name == name {
                for entry in std::fs::read_dir(&path)? {
                    let path = entry?.path();
                    let utf_8_path = Utf8Path::from_path(&path).expect("Deberia ser UTF-8");

                    if utf_8_path.is_file() {
                        if let Some(ext) = utf_8_path.extension() {
                            let file = DataSources::from_extension(ext)?;
                            files.push((path, file));
                        }
                    }
                }
            }
        }
    }

    info!("Escaneando los archivos disponibles... listo!");

    Ok(Folder {
        name: doc.name.clone(),
        files,
    })
}
