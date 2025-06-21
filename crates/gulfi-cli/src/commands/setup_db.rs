use camino::Utf8Path;
use color_eyre::owo_colors::OwoColorize;
use eyre::eyre;
use gulfi_common::{Document, MEMORY_DB_PATH};

use crate::CliError;

pub fn handle<P: AsRef<Utf8Path>>(
    db_path: P,
    docs: &[Document],
    doc: &str,
) -> Result<Document, CliError> {
    let db_path_str = db_path.as_ref().as_str();
    if db_path_str.trim() == MEMORY_DB_PATH {
        eprintln!(
            "You are running '{}' in a {}.",
            "Sync".cyan().bold(),
            "transient in-memory database".yellow().underline().bold()
        );

        return Err(CliError::Other(eyre!(
            "You should not sync on a transient database, it will dissapear immediately after"
        )));
    }

    let conn = gulfi_sqlite::get_vec_conn(db_path)?;

    let Some(doc) = docs.iter().find(|d| d.name == doc) else {
        let available_documents = docs
            .iter()
            .map(|x| x.name.clone())
            .collect::<Vec<_>>()
            .join(", ");

        return Err(CliError::Other(eyre!(
            "{} is not one of the available documents: [{available_documents}]",
            doc.bright_red()
        )));
    };

    gulfi_sqlite::insert_new_data(&conn, doc)?;

    Ok(doc.clone())
}
