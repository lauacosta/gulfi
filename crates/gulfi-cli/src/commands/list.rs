use gulfi_ingest::Document;

use crate::{CliError, Format};

pub fn handle(documents: &[Document], format: &Format) -> Result<(), CliError> {
    match format {
        Format::Pretty => {
            for doc in documents {
                println!("{doc}");
            }
        }
        Format::Json => println!("{}", serde_json::to_string_pretty(documents)?),
    }

    Ok(())
}
