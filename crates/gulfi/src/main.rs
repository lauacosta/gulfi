use clap::Parser;
use color_eyre::owo_colors::OwoColorize;
use eyre::eyre;
use gulfi::ApplicationSettings;
use gulfi::startup::run_server;
use gulfi_cli::{Cli, Commands, SyncStrategy};
use gulfi_common::Document;
use gulfi_helper::initialize_meta_file;
use gulfi_sqlite::{init_sqlite, insert_base_data, setup_sqlite, sync_fts_tnea, sync_vec_tnea};
use rusqlite::Connection;
use std::fs::File;
use tracing::{Level, debug, info, level_filters::LevelFilter};
use tracing_error::ErrorLayer;
use tracing_subscriber::{Registry, layer::SubscriberExt, util::SubscriberInitExt};
use tracing_tree::HierarchicalLayer;

fn main() -> eyre::Result<()> {
    let cli = Cli::parse();

    setup(cli.loglevel)?;

    let file = match File::open("meta.json") {
        Ok(file) => Ok(file),
        Err(_) => {
            initialize_meta_file()?;
            File::open("meta.json")
        }
    }?;

    let documents: Vec<Document> = serde_json::from_reader(file)
        .map_err(|err| eyre!("Error al parsear `meta.json`. {err}"))?;

    debug!(?documents);

    match cli.command {
        Commands::List => {
            println!("Documentos definidos en `meta.json`:");
            for doc in documents {
                let name = doc.name.to_uppercase();

                println!("{:<4}- {name}:", "");
                for field in doc.fields {
                    let field_name = field.name;
                    if field.vec_input && field.unique {
                        println!(
                            "{:<6}- {field_name} \t {}, {}",
                            "",
                            "vec_input".bright_blue().bold(),
                            "único".bright_magenta().bold(),
                        );
                    } else if field.vec_input {
                        println!(
                            "{:<6}- {field_name} \t {}",
                            "",
                            "vec_input".bright_blue().bold()
                        );
                    } else if field.unique {
                        println!(
                            "{:<6}- {field_name} \t {}",
                            "",
                            "único".bright_magenta().bold()
                        );
                    } else {
                        println!("{:<6}- {field_name}", "");
                    }
                }
            }
        }

        Commands::Serve {
            interface,
            port,
            open,
        } => {
            let configuration = ApplicationSettings::new(port, interface, open);

            debug!(?configuration);
            let rt = tokio::runtime::Runtime::new()?;

            rt.block_on(run_server(configuration))?
        }
        Commands::Sync {
            sync_strat,
            clean_slate,
            base_delay,
            document,
        } => {
            let base_delay = base_delay * 1000;
            let db = Connection::open(init_sqlite()?)?;

            let doc = match documents.iter().find(|doc| doc.name == document) {
                Some(doc) => doc,
                None => {
                    let available_documents = documents
                        .iter()
                        .map(|x| x.name.clone())
                        .collect::<Vec<_>>()
                        .join(", ");

                    return Err(eyre!(
                        "{} no es uno de los documentos disponibles: [{available_documents}]",
                        document.bright_red()
                    ));
                }
            };

            if clean_slate {
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

            setup_sqlite(&db, &doc)?;
            insert_base_data(&db, &doc)?;

            match sync_strat {
                SyncStrategy::Fts => sync_fts_tnea(&db, &doc),
                SyncStrategy::Vector => {
                    let rt = tokio::runtime::Runtime::new()?;
                    rt.block_on(sync_vec_tnea(&db, &doc, base_delay))?;
                }
                SyncStrategy::All => {
                    sync_fts_tnea(&db, &doc);
                    let rt = tokio::runtime::Runtime::new()?;
                    rt.block_on(sync_vec_tnea(&db, &doc, base_delay))?;
                }
            }

            info!(
                "Sincronización finalizada, tomó {} ms",
                start.elapsed().as_millis()
            );
        }
    }

    Ok(())
}

fn setup(loglevel: String) -> eyre::Result<()> {
    color_eyre::install()?;
    dotenvy::dotenv().map_err(|err| eyre!("El archivo .env no fue encontrado. err: {}", err))?;
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

    Registry::default()
        .with(LevelFilter::from_level(level))
        .with(
            HierarchicalLayer::new(2)
                .with_targets(true)
                .with_bracketed_fields(true)
                .with_ansi(true),
        )
        .with(ErrorLayer::default())
        .init();

    Ok(())
}
