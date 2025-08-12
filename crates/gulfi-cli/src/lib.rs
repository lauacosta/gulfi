pub mod clierror;
pub mod commands;
pub mod helper;

pub use clierror::*;
pub use gulfi_server::configuration::get_configuration;

use clap::{Parser, Subcommand, ValueEnum, command, crate_version};
use gulfi_server::configuration::Settings;
use std::{net::IpAddr, path::PathBuf};

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
    #[arg(long = "database-path")]
    pub db: Option<PathBuf>,

    /// Path to the metadata file for documents
    #[arg(long = "meta-file-path")]
    pub meta_file_path: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Command,
}

impl Cli {
    pub fn check_config() -> Result<(), CliError> {
        crate::commands::configuration::create_config_template()
    }

    pub fn merge_with_config(cli: Cli, config: &Settings) -> Cli {
        let db = cli.db.clone().or(Some(config.db_settings.db_path.clone()));
        let meta_file_path = cli
            .meta_file_path
            .clone()
            .or(Some(config.app_settings.meta_file_path.clone()));

        Cli {
            db,
            meta_file_path,
            ..cli
        }
    }
}

#[derive(Subcommand, Clone, Debug, PartialEq, Eq)]
pub enum Command {
    /// Starts the HTTP server.
    Serve {
        #[cfg(debug_assertions)]
        mode: Profile,

        /// Sets the IP address.
        #[clap(short = 'I', long)]
        interface: Option<IpAddr>,

        /// Sets the port.
        #[clap(short = 'P', long)]
        port: Option<u16>,

        /// Number of sqlite connections in the pool.
        #[clap(long)]
        pool_size: Option<usize>,

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
    List {
        #[arg(value_enum, long, default_value_t = Format::Pretty)]
        format: Format,
    },
    /// Adds a new document.
    Add,
    /// Deletes a document.
    Delete { document: String },
    /// Creates a new user in the database.
    CreateUser { username: String, password: String },
}

#[cfg(debug_assertions)]
#[derive(Debug, Clone, ValueEnum, PartialEq, Eq)]
pub enum Profile {
    Dev,
    Prod,
}

#[derive(Debug, Clone, ValueEnum, PartialEq, Eq)]
pub enum Format {
    Json,
    Pretty,
}

#[derive(Debug, Clone, ValueEnum, PartialEq, Eq)]
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
