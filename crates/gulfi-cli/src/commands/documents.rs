use camino::Utf8Path;
use fs_err::OpenOptions;
use gulfi_common::Document;

use crate::{CliError, helper};

pub fn add_document() -> Result<(), CliError> {
    helper::run_new().map_err(Into::into)
}

pub fn delete_document(name: &str, meta_file_path: &Utf8Path) -> Result<(), CliError> {
    let mut all_docs: Vec<Document> = if meta_file_path.exists() {
        let json_str = fs_err::read_to_string(meta_file_path)?;
        serde_json::from_str(&json_str).unwrap_or_default()
    } else {
        return Err(eyre::eyre!("No se ha encontrado un archivo `meta.json`.").into());
    };

    if let Some(pos) = all_docs
        .iter()
        .position(|doc| doc.name.to_lowercase() == name.to_lowercase())
    {
        all_docs.remove(pos);
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(meta_file_path)?;

        Ok(serde_json::to_writer_pretty(file, &all_docs)?)
    } else {
        Err(eyre::eyre!("No se encuentra ningun documento con nombre {name}.").into())
    }
}
