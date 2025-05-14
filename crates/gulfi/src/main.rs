use std::{fs::File, time::Instant};

use clap::{Parser, crate_name, crate_version};
use color_eyre::owo_colors::OwoColorize;
use eyre::eyre;
use gulfi_cli::{Cli, Command, SyncStrategy};
use gulfi_common::Document;
use gulfi_server::{ApplicationSettings, startup::run_server};
use gulfi_sqlite::{init_sqlite, insert_base_data, setup_sqlite, sync_fts_data, sync_vec_data};
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

    setup_tracing(&cli.loglevel)?;

    let file = match File::open("meta.json") {
        Ok(file) => Ok(file),
        Err(_) => {
            gulfi_cli::helper::initialize_meta_file()?;
            File::open("meta.json")
        }
    }?;

    let documents: Vec<Document> = serde_json::from_reader(file)
        .map_err(|err| eyre!("Error al parsear `meta.json`. {err}"))?;

    debug!(?documents);

    match cli.command() {
        Command::List => {
            println!("Documentos definidos en `meta.json`:");
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
            let start = std::time::Instant::now();
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
        } => {
            let base_delay = base_delay * 1000;
            let db_path = cli.db.clone();
            let db = init_sqlite(&db_path)?;

            let doc = match documents.iter().find(|doc| doc.name == document) {
                Some(doc) => doc,
                None => {
                    let available_documents = documents
                        .into_iter()
                        .map(|x| x.name)
                        .collect::<Vec<_>>()
                        .join(", ");

                    return Err(eyre!(
                        "{} no es uno de los documentos disponibles: [{available_documents}]",
                        document.bright_red()
                    ));
                }
            };

            if force {
                let exists: String = match db.query_row(
                    "select name from sqlite_master where type='table' and name=?",
                    [&document],
                    |row| row.get(0),
                ) {
                    Ok(msg) => msg,
                    Err(err) => {
                        return Err(eyre!(
                            "Es probable que la base de datos no esté creada. {}",
                            err
                        ));
                    }
                };

                if !exists.is_empty() {
                    db.execute(&format!("drop table {document}"), [])?;
                    db.execute(&format!("drop table {document}_raw"), [])?;
                    db.execute(&format!("drop table vec_{document}"), [])?;
                }
            }

            let start = std::time::Instant::now();
            setup_sqlite(&db, doc)?;
            insert_base_data(&db, doc)?;
            match sync_strat {
                SyncStrategy::Fts => {
                    let start = Instant::now();
                    let inserted = sync_fts_data(&db, doc);

                    eprintln!("{}", "-".repeat(100));
                    eprintln!(
                        "{inserted} registros fueron sincronizados en {} ({} ms).",
                        format!("fts_{}", doc.name).bright_cyan().bold(),
                        start.elapsed().as_millis(),
                    );
                }
                SyncStrategy::Vector => {
                    let rt = tokio::runtime::Runtime::new()?;

                    let start = Instant::now();
                    let (inserted, media) = rt.block_on(sync_vec_data(&db, doc, base_delay))?;

                    eprintln!("{}", "-".repeat(100));
                    eprintln!(
                        "{inserted} registros fueron sincronizados en {} ({} ms, media de {media} ms por chunk).",
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
                    let (inserted, media) = rt.block_on(sync_vec_data(&db, doc, base_delay))?;
                    let vec_elapsed = start.elapsed().as_millis();

                    eprintln!("{}", "-".repeat(100));

                    eprintln!(
                        "{inserted_fts} registros fueron sincronizados en {} ({fts_elapsed} ms).",
                        format!("fts_{}", doc.name).bright_cyan().bold(),
                    );

                    eprintln!(
                        "{inserted} registros fueron sincronizados en {} ({vec_elapsed} ms, media de {media} ms por chunk).",
                        format!("vec_{}", doc.name).bright_purple().bold(),
                    );
                }
            }

            eprintln!(
                "\n🎉 Sincronización finalizada, tomó {} ms.\n",
                start.elapsed().as_millis()
            );
        }
    }

    Ok(())
}

fn setup_tracing(loglevel: &str) -> eyre::Result<()> {
    color_eyre::install()?;

    if let Err(_) = dotenvy::dotenv() {
        eprintln!("El archivo {} no fue encontrado.", "\'env\'".green().bold());
    }

    let level = match loglevel.to_lowercase().trim() {
        "trace" => Level::TRACE,
        "debug" => Level::DEBUG,
        "info" => Level::INFO,
        _ => {
            return Err(eyre!(
                "Log Level desconocido, utiliza `INFO`, `DEBUG` o `TRACE`."
            ));
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

    tracing::subscriber::set_global_default(subscriber).unwrap();

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
        // let time = format!("~{}ms", elapsed.as_millis());
        let str = format!("{}", datetime.bright_blue());

        write!(w, "{}", str)?;
        Ok(())
    }
}
