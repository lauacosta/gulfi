use std::{fmt::Debug, path::Path};

use gulfi_common::Document;

use crate::{CliError, Format};

pub fn handle<P>(documents: &[Document], meta_path: P, format: &Format) -> Result<(), CliError>
where
    P: AsRef<Path> + Debug,
{
    println!("Document definitions in `{meta_path:#?}`:");
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
