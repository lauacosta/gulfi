pub mod clierror;
pub mod commands;
pub mod helper;
pub use clierror::*;
pub use gulfi_server::configuration::get_configuration;

use clap::{Parser, Subcommand, ValueEnum, command, crate_version};
use eyre::Result;
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

    /// TODO: Possibly support postgres.
    /// Path to the sqlite database
    #[arg(long = "database-path")]
    pub db: Option<PathBuf>,

    /// TODO: Pending review of maybe just removing this file.
    /// Path to the metadata file for documents
    #[arg(long = "meta-file-path")]
    pub meta_file_path: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Clone, Debug, PartialEq, Eq)]
pub enum Command {
    /// Starts the HTTP server.
    Serve {
        /// Sets the IP address for the HTTP server.
        #[clap(short = 'I', long)]
        interface: Option<IpAddr>,

        /// Sets the port for the HTTP server.
        #[clap(short = 'P', long)]
        port: Option<u16>,

        /// Number of sqlite connections in the pool.
        #[clap(long)]
        pool_size: Option<usize>,

        /// Opens the web interface in the default browser.
        #[arg(long, default_value = "false")]
        open: bool,

        /// TODO: Polish this interface.
        /// Connects to a observability tool (just Honeycomb for now)
        #[arg(long, default_value = "false")]
        telemetry: bool,
    },
    /// Updates the database.
    Sync {
        /// Name of the document to sync.
        document: String,
        /// Rewrites the database from scratch.
        #[arg(long, default_value = "false")]
        force: bool,

        /// Sets the strategy for updating. Possible values are Fts, Vector, Full and indicate
        /// wether it will synchronize the fts virtual table, the vector virtual table or both
        /// tables against the new data.
        #[arg(value_enum,  default_value_t = SyncStrategy::Fts)]
        sync_strat: SyncStrategy,

        // TODO: Open up to other alternatives to embedding, like local models, etc.
        /// Sets the base time for doing expontential backoff for the requests to OpenAI's API in ms.
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
    /// Starts the wizard to add a new document.
    Add,

    // TODO: Evaluate the need of the meta.json file instead of having everything in the database.
    /// Deletes a document, deleting it from the meta.json and the sqlite database.
    Delete { document: String },
}

impl Cli {
    pub fn merge_with_config(&mut self, config: &Settings) {
        self.db
            .get_or_insert_with(|| config.db_settings.path.clone());
        self.meta_file_path
            .get_or_insert_with(|| config.app_settings.meta_file_path.clone());
    }

    pub fn check_config() -> Result<(), CliError> {
        crate::commands::configuration::create_config_template()
    }
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
