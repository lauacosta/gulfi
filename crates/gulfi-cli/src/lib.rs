pub mod helper;

use clap::{Parser, Subcommand, ValueEnum, command, crate_version};
use std::net::IpAddr;
use thiserror::Error;

#[derive(Parser)]
#[command(version, about,  long_about = None, before_help = format!(r"
 _____       _  __ _ 
|  __ \     | |/ _(_)
| |  \/_   _| | |_ _ 
| | __| | | | |  _| |
| |_\ \ |_| | | | | |
 \____/\__,_|_|_| |_| {}

    @lauacosta/gulfi", crate_version!()
    ))
]
pub struct Cli {
    #[arg(long = "level", default_value = "INFO")]
    pub loglevel: String,

    /// Path to the sqlite database
    #[arg(long = "database-path", default_value = "./gulfi.db")]
    pub db: String,

    #[command(subcommand)]
    command: Option<Command>,
}

impl Cli {
    #[must_use]
    pub fn command(&self) -> Command {
        self.command.clone().unwrap_or(Command::List)
    }
}

#[derive(Subcommand, Clone, Debug)]
pub enum Command {
    /// Starts the HTTP server.
    Serve {
        #[cfg(debug_assertions)]
        #[arg(value_enum)]
        mode: Mode,
        /// Sets the IP address.
        #[clap(short = 'I', long, default_value = "127.0.0.1")]
        interface: IpAddr,

        /// Sets the port.
        #[clap(short = 'P', long, default_value_t = 3000)]
        port: u16,

        /// Opens the web interface.
        #[arg(long, default_value = "false")]
        open: bool,
    },
    /// Updates the database.
    Sync {
        document: String,

        /// Updates from scratch.
        #[arg(long, default_value = "false")]
        force: bool,

        /// Sets the strategy for updating.
        #[arg(value_enum,  default_value_t = SyncStrategy::Fts)]
        sync_strat: SyncStrategy,

        /// Sets the base time for backoff in requests in ms.
        #[arg(long, default_value_t = 2)]
        base_delay: u64,

        /// Sets the size of the chunks when splitting the entries for processing.
        #[arg(long, default_value_t = 1024)]
        chunk_size: usize,
    },
    /// Lists all defined documents.
    List,
    /// Adds a new document.
    Add,
    /// Deletes a document.
    Delete { document: String },
    /// Creates a new user in the database.
    CreateUser { username: String, password: String },
}

#[cfg(debug_assertions)]
#[derive(Debug, Clone, ValueEnum)]
pub enum Mode {
    Prod,
    Dev,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum SyncStrategy {
    Fts,
    Vector,
    All,
}

#[allow(unused)]
#[derive(Debug, Clone, ValueEnum)]
pub enum Cache {
    Enabled,
    Disabled,
}

#[derive(Error, Debug)]
pub enum CliError {
    #[error("Could not parse `meta.json`: {0}")]
    MetaParseError(#[from] serde_json::Error),

    #[error("Failed to open `meta.json`: {0}")]
    MetaOpenError(#[from] std::io::Error),

    #[error("SQLite error: {0}")]
    SqliteError(#[from] rusqlite::Error),

    // #[error("Password hashing failed: {0}")]
    // HashingError(#[from] password_hash::Error),
    #[error("Other: {0}")]
    Other(#[from] eyre::Report),
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//
//     // #[test]
//     // fn it_works() {
//     //     let result = add(2, 2);
//     //     assert_eq!(result, 4);
//     // }
// }
//
//
