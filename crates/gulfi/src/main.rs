#![allow(clippy::too_many_lines)]

use std::{fs::File, time::Instant};

use argon2::{Argon2, PasswordHasher};
use clap::{Parser, crate_name, crate_version};
use color_eyre::owo_colors::OwoColorize;
use eyre::eyre;
use gulfi::{MEMORY_DB_PATH, META_JSON_FILE, MILLISECONDS_MULTIPLIER, SEPARATOR_LINE};
use gulfi_cli::{Cli, CliError, Command, SyncStrategy};
use gulfi_common::Document;
use gulfi_server::{ApplicationSettings, startup::run_server};
use gulfi_sqlite::{init_sqlite, insert_base_data, setup_sqlite, sync_fts_data, sync_vec_data};
use password_hash::{SaltString, rand_core::OsRng};
use rusqlite::params;
use tracing::{Level, debug, level_filters::LevelFilter};
use tracing_error::ErrorLayer;
use tracing_subscriber::{Registry, fmt::Layer, layer::SubscriberExt};

#[cfg(debug_assertions)]
use eyre::Report;
#[cfg(debug_assertions)]
use gulfi_cli::Mode;
#[cfg(debug_assertions)]
use tokio::{process::Command as TokioCommand, try_join};

fn main() -> eyre::Result<()> {
    let cli = Cli::parse();
    setup_configuration(&cli.loglevel)?;

    if let Err(e) = run_cli(cli) {
        e.exit_with_tips();
    };

    Ok(())
}

fn run_cli(cli: Cli) -> Result<(), CliError> {
    let file = if let Ok(file) = File::open(META_JSON_FILE) {
        Ok(file)
    } else {
        gulfi_cli::helper::initialize_meta_file()?;
        File::open(META_JSON_FILE)
    }?;

    let documents: Vec<Document> = serde_json::from_reader(file)?;

    debug!(?documents);

    match cli.command() {
        Command::List => {
            println!("Document definitions in `meta.json`:");
            for doc in documents {
                println!("{doc}");
            }
        }
        Command::Add => {
            gulfi_cli::helper::run_new()?;
        }
        Command::Delete { document } => {
            gulfi_cli::helper::delete_document(&document)?;
        }
        Command::Serve {
            interface,
            port,
            open,
            #[cfg(debug_assertions)]
            mode,
        } => {
            let start = Instant::now();
            let name = crate_name!().to_owned();
            let version = crate_version!().to_owned();

            let configuration = ApplicationSettings::new(name, version, port, interface, open);

            debug!(?configuration);
            let rt = tokio::runtime::Runtime::new()?;

            #[cfg(debug_assertions)]
            match mode {
                Mode::Dev => {
                    let frontend_future = async {
                        TokioCommand::new("pnpm")
                            .arg("run")
                            .arg("dev")
                            .arg("--clearScreen=false")
                            .current_dir("./crates/gulfi-server/ui")
                            .stdout(std::process::Stdio::inherit())
                            .stderr(std::process::Stdio::inherit())
                            .spawn()
                            .map_err(Report::from)?
                            .wait()
                            .await
                            .map_err(Report::from)?;
                        Ok::<(), Report>(())
                    };

                    rt.block_on(async {
                        try_join!(run_server(configuration, start, documents), frontend_future)
                    })?;
                }
                Mode::Prod => rt.block_on(run_server(configuration, start, documents))?,
            }

            #[cfg(not(debug_assertions))]
            {
                rt.block_on(run_server(configuration, start, documents))?;
            }
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

            if db_path.trim() == MEMORY_DB_PATH {
                eprintln!(
                    "You are running '{}' in a {}.",
                    "Sync".cyan().bold(),
                    "transient in-memory database".yellow().underline().bold()
                );
                std::process::exit(1);
            }

            let db = init_sqlite(&db_path)?;

            let Some(doc) = documents.iter().find(|doc| doc.name == document) else {
                let available_documents = documents
                    .into_iter()
                    .map(|x| x.name)
                    .collect::<Vec<_>>()
                    .join(", ");

                return Err(CliError::Other(eyre!(
                    "{} is not one of the available documents: [{available_documents}]",
                    document.bright_red()
                )));
            };

            if force {
                let exists = db.query_row(
                    "select name from sqlite_master where type='table' and name=?",
                    [&document],
                    |row| row.get::<_, String>(0),
                )?;

                if !exists.is_empty() {
                    db.execute(&format!("drop table {document}"), [])?;
                    db.execute(&format!("drop table {document}_raw"), [])?;
                    db.execute(&format!("drop table vec_{document}"), [])?;
                }
            }

            let start = Instant::now();
            setup_sqlite(&db, doc)?;
            insert_base_data(&db, doc)?;
            match sync_strat {
                SyncStrategy::Fts => {
                    let start = Instant::now();
                    let inserted = sync_fts_data(&db, doc);

                    eprintln!("{SEPARATOR_LINE}");
                    eprintln!(
                        "{inserted} entries were synced in {} ({} ms).",
                        format!("fts_{}", doc.name).bright_cyan().bold(),
                        start.elapsed().as_millis(),
                    );
                }
                SyncStrategy::Vector => {
                    let rt = tokio::runtime::Runtime::new()?;

                    let start = Instant::now();
                    let (inserted, media) =
                        rt.block_on(sync_vec_data(&db, doc, base_delay, chunk_size))?;

                    eprintln!("{SEPARATOR_LINE}");
                    eprintln!(
                        "{inserted} entries were synced in {} ({} ms, average of {media} ms per chunk).",
                        format!("vec_{}", doc.name).bright_purple().bold(),
                        start.elapsed().as_millis(),
                    );
                }
                SyncStrategy::All => {
                    let start = Instant::now();
                    let inserted_fts = sync_fts_data(&db, doc);
                    let fts_elapsed = start.elapsed().as_millis();

                    let rt = tokio::runtime::Runtime::new()?;
                    let start = Instant::now();
                    let (inserted, media) =
                        rt.block_on(sync_vec_data(&db, doc, base_delay, chunk_size))?;
                    let vec_elapsed = start.elapsed().as_millis();

                    eprintln!("{SEPARATOR_LINE}");

                    eprintln!(
                        "{inserted_fts} entries were synced in {} ({fts_elapsed} ms).",
                        format!("fts_{}", doc.name).bright_cyan().bold(),
                    );

                    eprintln!(
                        "{inserted} entries were synced in {} ({vec_elapsed} ms, average of {media} ms per chunk).",
                        format!("vec_{}", doc.name).bright_purple().bold(),
                    );
                }
            }

            eprintln!(
                "\n🎉 Synchronization finished! took {} ms.\n",
                start.elapsed().as_millis()
            );
        }
        Command::CreateUser { username, password } => {
            let db_path = cli.db.clone();
            let db = init_sqlite(&db_path)?;

            db.execute_batch(
                "create table if not exists users (
                    id integer primary key autoincrement,
                    username text not null unique,
                    password_hash text not null,
                    auth_token text,
                    created_at datetime default current_timestamp,
                    updated_at datetime default current_timestamp
                )",
            )?;

            let salt = SaltString::generate(&mut OsRng);
            let argon2 = Argon2::default();
            let password_hash = argon2
                .hash_password(password.as_bytes(), &salt)
                .expect("TODO")
                .to_string();

            let updated = db.execute(
                "insert or replace into users(username, password_hash) values (?,?)",
                params![username, password_hash],
            )?;

            assert_eq!(updated, 1);

            println!("User {} was created", username.bold().bright_green());
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
                .with_timer(GulfiTimer::new()),
        )
        .with(ErrorLayer::default());

    tracing::subscriber::set_global_default(subscriber)?;

    Ok(())
}

use std::fmt;

struct GulfiTimer;

impl GulfiTimer {
    fn new() -> Self {
        Self
    }
}

impl tracing_subscriber::fmt::time::FormatTime for GulfiTimer {
    fn format_time(&self, w: &mut tracing_subscriber::fmt::format::Writer<'_>) -> fmt::Result {
        let datetime = chrono::Local::now().format("%H:%M:%S");
        let str = format!("{}", datetime.bright_blue());

        write!(w, "{str}")?;
        Ok(())
    }
}
