use camino::Utf8PathBuf;
use gulfi_common::Document;
use gulfi_server::{
    configuration::{Settings, get_configuration},
    startup::run_server,
    telemetry::{get_subscriber, init_subscriber},
};
use std::{net::IpAddr, time::Instant};

use crate::CliError;

#[cfg(debug_assertions)]
use eyre::Report;

#[cfg(debug_assertions)]
use crate::Profile;

#[cfg(debug_assertions)]
use tokio::{process::Command as TokioCommand, try_join};

pub struct ServerOverrides {
    interface: Option<IpAddr>,
    port: Option<u16>,
    db_path: Option<Utf8PathBuf>,
    pool_size: Option<usize>,
}

impl ServerOverrides {
    pub fn new(
        interface: Option<IpAddr>,
        port: Option<u16>,
        db_path: Option<Utf8PathBuf>,
        pool_size: Option<usize>,
    ) -> Self {
        Self {
            interface,
            port,
            db_path,
            pool_size,
        }
    }
    pub fn apply_to_config(self, config: &mut Settings) {
        if let Some(pool_size) = self.pool_size {
            config.db_settings.pool_size = pool_size;
        }
        if let Some(db_path) = self.db_path {
            config.db_settings.db_path = db_path;
        }
        if let Some(interface) = self.interface {
            config.app_settings.host = interface;
        }
        if let Some(port) = self.port {
            config.app_settings.port = port;
        }
    }
}

pub fn start_server(
    overrides: ServerOverrides,
    open: bool,
    documents: Vec<Document>,
    #[cfg(debug_assertions)] mode: &Profile,
) -> Result<(), CliError> {
    let start = Instant::now();

    let subscriber = get_subscriber("info,tokio=trace,runtime=trace".into());

    init_subscriber(subscriber);
    let mut configuration = get_configuration()?;
    overrides.apply_to_config(&mut configuration);

    let rt = tokio::runtime::Runtime::new()?;

    #[cfg(debug_assertions)]
    match mode {
        Profile::Dev => {
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
                try_join!(
                    run_server(configuration, start, documents, open),
                    frontend_future
                )
            })?;
        }
        Profile::Prod => rt.block_on(run_server(configuration, start, documents, open))?,
    }

    #[cfg(not(debug_assertions))]
    {
        rt.block_on(run_server(configuration, start, documents, open))?;
    }
    Ok(())
}
