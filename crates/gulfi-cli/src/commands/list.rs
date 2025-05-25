use gulfi_common::Document;

use crate::{CliError, Format};

pub fn handle(documents: &[Document], meta_path: &str, format: &Format) -> Result<(), CliError> {
    println!("Document definitions in `{meta_path}`:");
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
