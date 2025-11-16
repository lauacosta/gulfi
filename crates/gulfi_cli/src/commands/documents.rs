use crate::{CliError, helper};

pub fn add_document() -> Result<(), CliError> {
    helper::run_new().map_err(Into::into)
}

pub fn delete_document(doc: &str) -> Result<(), CliError> {
    helper::delete_document(doc).map_err(Into::into)
}
