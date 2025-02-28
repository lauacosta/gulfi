use clap::Parser;
use eyre::eyre;
use gulfi::ApplicationSettings;
use gulfi::startup::run_server;
use gulfi_cli::{Cli, Commands, SyncStrategy};
use gulfi_common::Source;
use gulfi_helper::initialize_meta_file;
use gulfi_sqlite::{init_sqlite, insert_base_data, setup_sqlite, sync_fts_tnea, sync_vec_tnea};
use rusqlite::Connection;
use std::fs::File;
use tracing::{Level, debug, info, level_filters::LevelFilter};
use tracing_error::ErrorLayer;
use tracing_subscriber::{
    Registry,
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
};
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

    let records: Vec<Source> = serde_json::from_reader(file)
        .map_err(|err| eyre!("Error al parsear `meta.json`. {err}"))?;

    debug!(?records);

    match cli.command {
        Commands::Serve {
            interface,
            port,
            // cache,
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
        } => {
            let base_delay = base_delay * 1000;
            let db = Connection::open(init_sqlite()?)?;

            if clean_slate {
                let exists: String = match db.query_row(
                    "select name from sqlite_master where type='table' and name=?",
                    ["tnea"],
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
                    db.execute("drop table tnea", [])?;
                    db.execute("drop table tnea_raw", [])?;
                    db.execute("drop table vec_tnea", [])?;
                }
            }

            let start = std::time::Instant::now();

            setup_sqlite(&db, &records)?;
            insert_base_data(&db, &records)?;

            match sync_strat {
                SyncStrategy::Fts => sync_fts_tnea(&db, &records),
                SyncStrategy::Vector => {
                    let rt = tokio::runtime::Runtime::new()?;
                    rt.block_on(sync_vec_tnea(&db, &records, base_delay))?;
                }
                SyncStrategy::All => {
                    sync_fts_tnea(&db, &records);
                    let rt = tokio::runtime::Runtime::new()?;
                    rt.block_on(sync_vec_tnea(&db, &records, base_delay))?;
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
