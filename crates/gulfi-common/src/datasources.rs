use std::{
    fmt::{Debug, Display, Formatter},
    fs::{DirBuilder, metadata},
    path::{Path, PathBuf},
};

use camino::Utf8Path;
use color_eyre::owo_colors::OwoColorize;
use eyre::eyre;
use serde::{Deserialize, Deserializer, Serialize};

use tracing::{error, info, warn};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Document {
    #[serde(deserialize_with = "to_lowercase")]
    pub name: String,
    pub fields: Vec<Field>,
}

impl Display for Document {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let name = self.name.to_uppercase();
        writeln!(f, "{:<4}- {name}:", "")?;

        for field in &self.fields {
            let field_name = &field.name;
            let formatted = if field.vec_input && field.unique {
                format!(
                    "{:<6}- {field_name} \t {}, {}",
                    "",
                    "vec_input".bright_blue().bold(),
                    "único".bright_magenta().bold(),
                )
            } else if field.vec_input {
                format!(
                    "{:<6}- {field_name} \t {}",
                    "",
                    "vec_input".bright_blue().bold()
                )
            } else if field.unique {
                format!(
                    "{:<6}- {field_name} \t {}",
                    "",
                    "único".bright_magenta().bold()
                )
            } else {
                format!("{:<6}- {field_name}", "")
            };

            writeln!(f, "{}", formatted)?;
        }

        Ok(())
    }
}

impl Document {
    pub fn generate_vec_input(&self) -> String {
        let mut result = String::from("'  '");
        for i in &self.fields {
            if i.vec_input {
                result.push_str(&format!(" || {} || '  '", i.name));
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

pub fn parse_sources<T: AsRef<Path> + Debug>(path: T) -> eyre::Result<Vec<(PathBuf, DataSources)>> {
    let mut datasources = Vec::new();

    match metadata(&path) {
        Err(err) => {
            error!("El directorio `{path:?}` no existe!: {err}");
            info!("Para solucionarlo, se creara un directorio con ese path");
            DirBuilder::new().recursive(true).create(&path)?;
        }
        Ok(metadata) => {
            if metadata.is_dir() {
                let entries = std::fs::read_dir(&path).expect("Deberia poder leer el directorio");
                if entries.into_iter().count() == 0 {
                    warn!("El directorio {path:?}` existe, pero no tiene archivos.");
                }
            } else {
                error!("`{path:?}` no es un directorio!");
                info!(
                    "Para solucionarlo, cree un directorio en `datasources` con el nombre de su documento."
                );
                return Err(eyre!("No es un directorio"));
            }
        }
    }

    for entry in std::fs::read_dir(&path)? {
        let path = entry?.path();
        let utf_8_path = Utf8Path::from_path(&path).expect("Deberia ser UTF-8");

        if utf_8_path.is_file() {
            if let Some(ext) = utf_8_path.extension() {
                let file = DataSources::from_extension(ext)?;
                datasources.push((path, file));
            }
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
