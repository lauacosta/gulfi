use std::fs::File;

use clap::Parser;
use eyre::eyre;
use gulfi::startup::run_server;
use gulfi_cli::{Cli, Commands, Model, SyncStrategy};
use gulfi_common::Document;
use gulfi_configuration::ApplicationSettings;
use gulfi_helper::{add_document_to_meta_file, initialize_meta_file};
use gulfi_openai::embed_single;
use gulfi_sqlite::{init_sqlite, insert_base_data, setup_sqlite, sync_fts_tnea, sync_vec_tnea};
use rusqlite::Connection;
use tracing::{Level, debug, info, level_filters::LevelFilter};
use tracing_error::ErrorLayer;
use tracing_subscriber::{Registry, layer::SubscriberExt, util::SubscriberInitExt};
use tracing_tree::HierarchicalLayer;

fn main() -> eyre::Result<()> {
    let cli = Cli::parse();

    setup(cli.loglevel)?;

    let file = match File::open("meta.json") {
        Ok(f) => Ok(f),
        Err(_) => {
            initialize_meta_file()?;
            File::open("meta.json")
        }
    }?;

    let documents: Vec<Document> = serde_json::from_reader(file)
        .map_err(|err| eyre!("Error al parsear `meta.json`. {err}"))?;

    debug!(?documents);

    match cli.command {
        Commands::Serve {
            interface,
            port,
            cache,
            open,
        } => {
            let configuration = ApplicationSettings::new(port, interface, cache, open);

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
            let document = document.trim().to_lowercase();

            if let Some(doc) = documents
                .iter()
                .find(|d| d.name.trim().to_lowercase() == document)
            {
                let db = Connection::open(init_sqlite()?)?;
                if clean_slate {
                    let exists: String = match db.query_row(
                        "select name from sqlite_master where type='table' and name=?",
                        [doc.name.clone()],
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
                        db.execute(&format!("drop table {}", doc.name), [])?;
                        db.execute(&format!("drop table {}_raw", doc.name), [])?;
                        db.execute(&format!("drop table vec_{}", doc.name), [])?;
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
            } else {
                println!("Este documento no existe!");
            }
        }
        Commands::Embed { input, model } => match model {
            Model::OpenAI => {
                let client = reqwest::Client::new();
                let rt = tokio::runtime::Runtime::new()?;
                let output = rt.block_on(embed_single(input, &client))?;
                println!("{output:?}");
            }
            Model::Local => {
                todo!()
            }
        },
        Commands::Add => add_document_to_meta_file(documents)?,
        Commands::List => {
            println!("{}", serde_json::to_string_pretty(&documents)?);
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
                .with_bracketed_fields(true),
        )
        .with(ErrorLayer::default())
        .init();
    Ok(())
}
