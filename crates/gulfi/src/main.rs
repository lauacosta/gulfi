#![allow(clippy::too_many_lines)]

use camino::Utf8PathBuf;
use clap::Parser;
use fs_err::File;
use gulfi_cli::commands::server::ServerOverrides;
use gulfi_cli::{Cli, CliError, Command, ExitOnError, helper::initialize_meta_file};
use gulfi_cli::{MigrationCommands, commands, get_configuration};
use gulfi_common::Document;
use gulfi_common::MILLISECONDS_MULTIPLIER;
use std::io::{Error, ErrorKind};
use std::path::Path;
use std::time::Instant;

fn main() -> eyre::Result<()> {
    color_eyre::install()?;
    let cli = Cli::parse();

    if let Err(e) = run_cli(cli) {
        e.exit_with_tips();
    }

    Ok(())
}

fn run_cli(cli: Cli) -> Result<(), CliError> {
    if !Path::new("configuration").is_dir() {
        eprintln!("Configuration directory missing, creating basic config!");
        commands::configuration::create_config_template().or_exit();
    }

    let cli = Cli::merge_with_config(cli, &get_configuration()?);
    let db_path = cli.db.clone().expect("TODO: remove this clone");

    let (meta_file, documents) = load_meta_docs(&cli)?;

    match cli.command() {
        Command::List { format } => {
            commands::list::handle(&documents, meta_file, &format).or_exit();
        }

        Command::Add => commands::documents::add_document().or_exit(),
        Command::Delete { document } => {
            commands::documents::delete_document(&document, &meta_file).or_exit();
        }

        Command::Migration { command } => match command {
            MigrationCommands::Generate => commands::migrations::generate(&documents).or_exit(),
            MigrationCommands::Migrate { dry_run } => {
                commands::migrations::migrate(db_path, dry_run).or_exit();
            }
            MigrationCommands::Status => commands::migrations::status(db_path).or_exit(),
            MigrationCommands::Fresh { dry_run } => {
                commands::migrations::fresh(db_path, dry_run).or_exit();
            }
            MigrationCommands::Create { name } => commands::migrations::create(name).or_exit(),
        },
        Command::Serve {
            interface,
            port,
            open,
            pool_size,
            #[cfg(debug_assertions)]
            mode,
        } => {
            let db_path = cli.db.clone();
            let overrides = ServerOverrides::new(interface, port, db_path, pool_size);

            #[cfg(debug_assertions)]
            commands::server::start_server(overrides, open, documents, &mode)?;

            #[cfg(not(debug_assertions))]
            commands::server::start_server(overrides, open, documents)?;
        }
        Command::Sync {
            sync_strat,
            base_delay,
            document,
            chunk_size,
        } => {
            let db_path = cli.db.as_ref().expect("db file missing");

            let base_delay = base_delay * MILLISECONDS_MULTIPLIER;

            let start = Instant::now();
            let doc = commands::setup_db::handle(db_path, &documents, &document)?;

            commands::sync::handle_update(db_path, &doc, &sync_strat, base_delay, chunk_size)?;

            eprintln!(
                "\n🎉 Synchronization finished! took {} ms.\n",
                start.elapsed().as_millis()
            );
        }
        Command::CreateUser { username, password } => {
            let db_path = cli.db.as_ref().expect("db file missing");

            commands::users::create_user(db_path, &username, &password).or_exit();
        }
    }

    Ok(())
}

fn load_meta_docs(cli: &Cli) -> Result<(Utf8PathBuf, Vec<Document>), CliError> {
    let meta_file = cli.meta_file_path.clone().ok_or_else(|| {
        CliError::MetaOpenError(Error::new(ErrorKind::NotFound, "meta file not found"))
    })?;

    let file = if let Ok(file) = File::open(&meta_file) {
        Ok(file)
    } else {
        initialize_meta_file()?;
        File::open(&meta_file)
    }?;

    Ok((
        meta_file,
        serde_json::from_reader::<_, Vec<Document>>(file)?,
    ))
}
