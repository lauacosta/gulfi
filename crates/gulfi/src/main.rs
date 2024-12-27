use clap::Parser;
use eyre::eyre;
use gulfi::startup;
use gulfi_cli::{Cli, Commands, Model, SyncStrategy};
use gulfi_configuration::{ApplicationSettings, Template};
use gulfi_openai::embed_single;
use gulfi_sqlite::{
    init_sqlite, insert_base_data, setup_sqlite, sync_fts_tnea, sync_vec_tnea,
};
use rusqlite::Connection;
use tracing::{Level, debug, info, level_filters::LevelFilter};
use tracing_error::ErrorLayer;
use tracing_subscriber::{Registry, layer::SubscriberExt, util::SubscriberInitExt};
use tracing_tree::HierarchicalLayer;

fn main() -> eyre::Result<()> {
    color_eyre::install()?;
    dotenvy::dotenv().map_err(|err| eyre!("El archivo .env no fue encontrado. err: {}", err))?;

    let cli = Cli::parse();
    let level = match cli.loglevel.to_lowercase().trim() {
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

    let template = std::env::var("TEMPLATE")
        .map_err(|err| eyre!("Hubo un error al leer la variable de entorno `TEMPLATE` {err}."))?;

    let template = Template::try_from(template)
        .map_err(|err| eyre!("Hubo un error al parsear el template {err}"))?;

    match cli.command {
        Commands::Serve {
            interface,
            port,
            cache,
            open,
        } => {
            let configuration = ApplicationSettings::new(port, interface, cache, open);

            debug!("{:?}", &configuration);
            let rt = tokio::runtime::Runtime::new()?;

            rt.block_on(startup::run_server(configuration))?
        }
        Commands::Sync {
            sync_strat,
            clean_slate,
            base_delay, // model,
        } => {
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

            setup_sqlite(&db)?;
            insert_base_data(&db, &template)?;

            match sync_strat {
                SyncStrategy::Fts => sync_fts_tnea(&db),
                SyncStrategy::Vector => {
                    let rt = tokio::runtime::Runtime::new()?;
                    rt.block_on(sync_vec_tnea(&db, base_delay))?;
                }
                SyncStrategy::All => {
                    sync_fts_tnea(&db);
                    let rt = tokio::runtime::Runtime::new()?;
                    rt.block_on(sync_vec_tnea(&db, base_delay))?;
                }
            }

            info!(
                "Sincronización finalizada, tomó {} ms",
                start.elapsed().as_millis()
            );
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
    }

    Ok(())
}