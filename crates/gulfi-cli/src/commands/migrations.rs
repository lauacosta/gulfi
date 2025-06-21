use camino::Utf8PathBuf;
use gulfi_common::Document;

use crate::CliError;

pub fn generate(docs: &[Document]) -> Result<(), CliError> {
    Ok(gulfi_sqlite::migrations::generate(docs)?)
}

pub fn migrate(db_path: Utf8PathBuf, dry_run: bool) -> Result<(), CliError> {
    Ok(gulfi_sqlite::migrations::migrate(db_path, dry_run)?)
}

pub fn fresh(db_path: Utf8PathBuf, dry_run: bool) -> Result<(), CliError> {
    Ok(gulfi_sqlite::migrations::fresh(db_path, dry_run)?)
}

pub fn status(db_path: Utf8PathBuf) -> Result<(), CliError> {
    Ok(gulfi_sqlite::migrations::status(db_path)?)
}

pub fn create(name: String) -> Result<(), CliError> {
    gulfi_sqlite::migrations::create_migration(name);
    Ok(())
}
