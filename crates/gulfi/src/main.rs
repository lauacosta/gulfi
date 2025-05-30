#![allow(clippy::too_many_lines)]

use std::{fs::File, time::Instant};

use clap::Parser;
use color_eyre::owo_colors::OwoColorize;
use eyre::eyre;
use gulfi::GulfiTimer;
use gulfi_cli::commands;
use gulfi_cli::{Cli, CliError, Command, ExitOnError, helper::initialize_meta_file};
use gulfi_common::Document;
use gulfi_common::{META_JSON_FILE, MILLISECONDS_MULTIPLIER};
use tracing::{Level, debug, level_filters::LevelFilter};
use tracing_error::ErrorLayer;
use tracing_subscriber::fmt;
use tracing_subscriber::{Registry, fmt::Layer, layer::SubscriberExt};

fn main() -> eyre::Result<()> {
    let cli = Cli::parse();
    setup_configuration(&cli.loglevel)?;

    if let Err(e) = run_cli(&cli) {
        e.exit_with_tips();
    }

    Ok(())
}

fn run_cli(cli: &Cli) -> Result<(), CliError> {
    let file = if let Ok(file) = File::open(META_JSON_FILE) {
        Ok(file)
    } else {
        initialize_meta_file()?;
        File::open(META_JSON_FILE)
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
            #[cfg(debug_assertions)]
            commands::server::start_server(interface, port, open, pool_size, documents, &mode)?;
            #[cfg(not(debug_assertions))]
            commands::server::start_server(interface, port, open, pool_size, documents)?;
        }
        Command::Sync {
            sync_strat,
            force,
            base_delay,
            document,
            chunk_size,
        } => {
            let base_delay = base_delay * MILLISECONDS_MULTIPLIER;
            let db_path = cli.db.clone();

            let start = Instant::now();
            let doc = commands::setup_db::handle(&db_path, &documents, &document, force)?;

            commands::sync::handle_update(&db_path, &doc, &sync_strat, base_delay, chunk_size)?;

            eprintln!(
                "\n🎉 Synchronization finished! took {} ms.\n",
                start.elapsed().as_millis()
            );
        }
        Command::CreateUser { username, password } => {
            commands::users::create_user(&cli.db, &username, &password).or_exit();
        }
    }

    Ok(())
}

fn setup_configuration(loglevel: &str) -> eyre::Result<()> {
    color_eyre::install()?;

    if dotenvy::dotenv().is_err() {
        eprintln!("{} was not found.", "\'env\'".green().bold());
    }

    let level = match loglevel.to_lowercase().trim() {
        "trace" => Level::TRACE,
        "debug" => Level::DEBUG,
        "info" => Level::INFO,
        _ => {
            return Err(eyre!("unknown log level, use `INFO`, `DEBUG` or `TRACE`."));
        }
    };

    let subscriber = Registry::default()
        .with(LevelFilter::from_level(level))
        .with(
            Layer::new()
                .compact()
                .with_ansi(true)
                .with_timer(GulfiTimer::new())
                .with_span_events(fmt::format::FmtSpan::FULL),
        )
        .with(ErrorLayer::default());

    tracing::subscriber::set_global_default(subscriber)?;

    Ok(())
}
