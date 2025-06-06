#![allow(clippy::too_many_lines)]

use std::io::{Error, ErrorKind};
use std::{fs::File, time::Instant};

use clap::Parser;
use gulfi_cli::commands::server::ServerOverrides;
use gulfi_cli::{Cli, CliError, Command, ExitOnError, helper::initialize_meta_file};
use gulfi_cli::{commands, get_configuration};
use gulfi_common::Document;
use gulfi_common::{META_JSON_FILE, MILLISECONDS_MULTIPLIER};
use tracing::debug;

fn main() -> eyre::Result<()> {
    color_eyre::install()?;
    let cli = Cli::parse();

    if let Err(e) = run_cli(&cli) {
        e.exit_with_tips();
    }

    Ok(())
}

fn run_cli(cli: &Cli) -> Result<(), CliError> {
    let meta_file = cli.meta_file_path.clone().ok_or_else(|| {
        CliError::MetaOpenError(Error::new(ErrorKind::NotFound, "meta file not found"))
    })?;

    let file = if let Ok(file) = File::open(&meta_file) {
        Ok(file)
    } else {
        initialize_meta_file()?;
        File::open(&meta_file)
    }?;

    let documents: Vec<Document> = serde_json::from_reader(file)?;

    debug!(?documents);

    match cli.command() {
        Command::List { format } => {
            commands::list::handle(&documents, META_JSON_FILE, &format).or_exit();
        }

        Command::Add => commands::documents::add_document().or_exit(),
        Command::Delete { document } => commands::documents::delete_document(&document).or_exit(),
        Command::Serve {
            interface,
            port,
            open,
            pool_size,
            #[cfg(debug_assertions)]
            mode,
        } => {
            let db_path = cli.db.clone();

            #[cfg(debug_assertions)]
            let overrides = ServerOverrides::new(interface, port, db_path, pool_size);
            commands::server::start_server(overrides, open, documents, &mode)?;

            #[cfg(not(debug_assertions))]
            commands::server::start_server(overrides, open, documents)?;
        }
        Command::Sync {
            sync_strat,
            force,
            base_delay,
            document,
            chunk_size,
        } => {
            let configuration = get_configuration()?;
            let db_path = cli
                .db
                .as_ref()
                .unwrap_or(&configuration.db_settings.db_path);

            let base_delay = base_delay * MILLISECONDS_MULTIPLIER;

            let start = Instant::now();
            let doc = commands::setup_db::handle(db_path, &documents, &document, force)?;

            commands::sync::handle_update(db_path, &doc, &sync_strat, base_delay, chunk_size)?;

            eprintln!(
                "\n🎉 Synchronization finished! took {} ms.\n",
                start.elapsed().as_millis()
            );
        }
        Command::CreateUser { username, password } => {
            let configuration = get_configuration()?;
            let db_path = cli
                .db
                .as_ref()
                .unwrap_or(&configuration.db_settings.db_path);

            commands::users::create_user(db_path, &username, &password).or_exit();
        }
        Command::Init => {
            commands::configuration::create_config_template().or_exit();
        }
    }

    Ok(())
}
