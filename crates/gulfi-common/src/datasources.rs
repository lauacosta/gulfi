use camino::{Utf8Path, Utf8PathBuf};
use color_eyre::owo_colors::OwoColorize;
use eyre::eyre;
use fs_err::metadata;
use serde::{Deserialize, Deserializer, Serialize};
use std::fmt::Write as _;
use std::fmt::{Debug, Display, Formatter};
use std::fs::DirBuilder;

use tracing::{error, info, warn};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Document {
    #[serde(deserialize_with = "to_lowercase")]
    pub name: String,
    pub fields: Vec<Field>,
}

impl Display for Document {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // Header with document name
        let name = self.name.to_uppercase();
        writeln!(
            f,
            "╭─ {} {}",
            "📄".bright_yellow(),
            name.bright_cyan().bold()
        )?;

        // Field count summary
        let field_count = self.fields.len();
        let vec_count = self.fields.iter().filter(|f| f.vec_input).count();
        let unique_count = self.fields.iter().filter(|f| f.unique).count();

        writeln!(
            f,
            "│  {} {} fields • {} vectorized • {} unique",
            "📊".bright_blue(),
            field_count.bright_white().bold(),
            vec_count.bright_blue().bold(),
            unique_count.bright_magenta().bold()
        )?;

        writeln!(f, "│")?;

        // Fields
        for (i, field) in self.fields.iter().enumerate() {
            let is_last = i == self.fields.len() - 1;
            let connector = if is_last { "╰─" } else { "├─" };

            // Field name with proper alignment
            let field_name = format!("{:<20}", field.name);

            // Build badges
            let mut badges = Vec::new();
            if field.vec_input {
                badges.push("🔍 vec".bright_blue().bold().to_string());
            }
            if field.unique {
                badges.push("⭐ único".bright_magenta().bold().to_string());
            }

            let badge_str = if badges.is_empty() {
                String::new()
            } else {
                format!(" {}", badges.join(" "))
            };

            writeln!(
                f,
                "{} {} {}{}",
                connector.bright_white(),
                field_name.bright_green(),
                "│".bright_white().dimmed(),
                badge_str
            )?;
        }

        Ok(())
    }
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
            _ => return Err(eyre!("unknown file extension: {ext}")),
        };

        Ok(file)
    }
}

pub fn parse_sources<T>(path: T) -> eyre::Result<Vec<(Utf8PathBuf, DataSources)>>
where
    T: AsRef<Utf8Path> + Debug,
{
    let mut datasources = Vec::new();
    let path_ref = path.as_ref();

    match metadata(path_ref) {
        Err(err) => {
            error!("Directory `{path:?}` doesn't exists!: {err}");
            info!("To fix it, create the directory.");
            DirBuilder::new().recursive(true).create(path_ref)?;
        }
        Ok(metadata) => {
            if metadata.is_dir() {
                let entries =
                    fs_err::read_dir(path.as_ref()).expect("Should be able to read the directory");
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

    for entry in fs_err::read_dir(path.as_ref())? {
        let path = entry?.path();
        let utf_8_path = Utf8Path::from_path(&path)
            .expect("Should be UTF-8")
            .to_owned();

        if utf_8_path.is_file() {
            if let Some(ext) = utf_8_path.extension() {
                let file = DataSources::from_extension(ext)?;
                datasources.push((utf_8_path, file));
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
