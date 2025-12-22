use std::io::IsTerminal;

use gulfi_ingest::Document;

use crate::{CliError, MessageFormat};

// TODO: Does it have any real use?
pub fn handle(documents: &[Document], message_format: MessageFormat) -> Result<(), CliError> {
    let emit_json = matches!(message_format, MessageFormat::Json)
    // Doesnt guarantee ansi_support but I dont really care.
    || !std::io::stdout().is_terminal();

    if emit_json {
        println!("{}", serde_json::to_string(documents)?)
    } else {
        for doc in documents {
            println!("{doc:?}");
        }
    }
    Ok(())
}
