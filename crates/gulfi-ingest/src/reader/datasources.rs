use std::borrow::Borrow;
use std::fmt::Write as _;
use std::{
    fmt::Debug,
    fs::{DirBuilder, metadata},
    path::{Path, PathBuf},
};

use camino::Utf8Path;
use eyre::eyre;
use serde::{Deserialize, Deserializer, Serialize};

use tracing::{error, info, warn};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Document {
    #[serde(deserialize_with = "to_lowercase")]
    pub name: String,
    pub fields: Vec<Field>,
}

impl Document {
    pub fn generate_vec_input(&self) -> String {
        let mut result = String::from("'  '");
        for i in &self.fields {
            if i.vec_input {
                let _ = write!(result, " || {} || '  '", i.name);
            }
        }

        result
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Field {
    pub name: String,
    pub vec_input: bool,
    pub unique: bool,
}

impl AsRef<str> for Field {
    fn as_ref(&self) -> &str {
        &self.name
    }
}

impl Borrow<str> for Field {
    fn borrow(&self) -> &str {
        &self.name
    }
}

#[derive(Debug, PartialEq)]
pub enum Filetype {
    Csv,
    Json,
}

impl Filetype {
    pub fn from_extension(ext: &str) -> eyre::Result<Self> {
        let file = match ext {
            "csv" => Filetype::Csv,
            "json" => Filetype::Json,
            _ => return Err(eyre!("unknown file extension: {ext}")),
        };

        Ok(file)
    }
}

pub fn parse_sources<T: AsRef<Path> + Debug>(path: T) -> eyre::Result<Vec<(PathBuf, Filetype)>> {
    let mut datasources = Vec::new();

    match metadata(&path) {
        Err(err) => {
            error!("Directory `{path:?}` doesn't exists!: {err}");
            info!("To fix it, create the directory.");
            DirBuilder::new().recursive(true).create(&path)?;
        }
        Ok(metadata) => {
            if metadata.is_dir() {
                let entries =
                    std::fs::read_dir(&path).expect("Should be able to read the directory");
                if entries.into_iter().count() == 0 {
                    warn!("Directory {path:?}` exists but is empty.");
                }
            } else {
                error!("`{path:?}` is not a directory!");
                info!(
                    "To fix it, create a directory in `datasources` with the name of your document or use the CLI command."
                );
                return Err(eyre!("Not a directory"));
            }
        }
    }

    for entry in std::fs::read_dir(&path)? {
        let path = entry?.path();
        let utf_8_path = Utf8Path::from_path(&path).expect("Should be UTF-8");

        if utf_8_path.is_file()
            && let Some(ext) = utf_8_path.extension()
        {
            let file = Filetype::from_extension(ext)?;
            datasources.push((path, file));
        }
    }

    Ok(datasources)
}

fn to_lowercase<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    Ok(s.to_lowercase())
}
