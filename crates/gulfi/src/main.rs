use std::io::{Error, ErrorKind};
use std::path::PathBuf;
use std::{fs::File, time::Instant};

use clap::Parser;
use gulfi_cli::commands::server::ServerOverrides;
use gulfi_cli::{Cli, CliError, Command, ExitOnError, helper::initialize_meta_file};
use gulfi_cli::{commands, get_configuration};
use gulfi_ingest::Document;

fn main() -> eyre::Result<()> {
    color_eyre::install()?;
    Cli::check_config()?;

    if let Err(e) = run_cli(Cli::parse()) {
        e.exit_with_tips();
    }

    Ok(())
}

fn run_cli(mut cli: Cli) -> Result<(), CliError> {
    cli.merge_with_config(&get_configuration()?);

    let (_, documents) = load_meta_docs(&cli)?;

    match cli.command {
        Command::List { format } => {
            commands::list::handle(&documents, &format).or_exit();
        }

        Command::Add => commands::documents::add_document().or_exit(),
        Command::Delete { document } => commands::documents::delete_document(&document).or_exit(),
        Command::Serve {
            interface,
            port,
            open,
            pool_size,
            telemetry,
        } => {
            let db_path = cli.db.clone();
            let overrides = ServerOverrides::new(interface, port, db_path, pool_size);

            commands::server::start_server(overrides, open, telemetry, documents)?;
        }
        Command::Sync {
            sync_strat,
            force,
            base_delay,
            document,
            chunk_size,
        } => {
            let db_path = cli.db.as_ref().expect("db file missing");

            let base_delay = base_delay * 1000;

            let start = Instant::now();
            let doc = commands::setup_db::handle(db_path, &documents, &document, force)?;

            commands::sync::handle_update(db_path, &doc, &sync_strat, base_delay, chunk_size)?;

            eprintln!(
                "\n🎉 Synchronization finished! took {} ms.\n",
                start.elapsed().as_millis()
            );
        }
    }

    Ok(())
}

fn load_meta_docs(cli: &Cli) -> Result<(PathBuf, Vec<Document>), CliError> {
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
