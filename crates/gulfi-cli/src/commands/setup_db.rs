use color_eyre::owo_colors::OwoColorize;
use eyre::eyre;
use gulfi_common::{Document, MEMORY_DB_PATH};
use gulfi_sqlite::{insert_base_data, setup_sqlite, spawn_vec_connection};

use crate::CliError;

pub fn handle(
    db_path: &str,
    docs: &[Document],
    doc: &str,
    force: bool,
) -> Result<Document, CliError> {
    if db_path.trim() == MEMORY_DB_PATH {
        eprintln!(
            "You are running '{}' in a {}.",
            "Sync".cyan().bold(),
            "transient in-memory database".yellow().underline().bold()
        );

        return Err(CliError::Other(eyre!(
            "You should not sync on a transient database, it will dissapear immediately after"
        )));
    }

    let conn = spawn_vec_connection(db_path)?;

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

    let doc_name = doc.name.clone();

    if force {
        let exists = conn.query_row(
            "select name from sqlite_master where type='table' and name=?",
            [&doc_name],
            |row| row.get::<_, String>(0),
        )?;

        if !exists.is_empty() {
            conn.execute(&format!("drop table {doc_name}"), [])?;
            conn.execute(&format!("drop table {doc_name}_raw"), [])?;
            conn.execute(&format!("drop table vec_{doc_name}"), [])?;
        }
    }

    setup_sqlite(&conn, doc)?;
    insert_base_data(&conn, doc)?;

    Ok(doc.clone())
}
