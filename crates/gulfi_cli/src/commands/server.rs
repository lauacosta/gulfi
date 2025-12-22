use gulfi_ingest::Document;
use gulfi_server::{
    configuration::{Settings, get_configuration},
    startup::run_server,
    telemetry::{get_subscriber, init_subscriber},
};
use std::{net::IpAddr, path::PathBuf, time::Instant};

use crate::CliError;

pub struct ServerOverrides {
    interface: Option<IpAddr>,
    port: Option<u16>,
    db_path: Option<PathBuf>,
    pool_size: Option<usize>,
}

impl ServerOverrides {
    pub fn new(
        interface: Option<IpAddr>,
        port: Option<u16>,
        db_path: Option<PathBuf>,
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
            config.db_settings.path = db_path;
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
    telemetry: bool,
    documents: Vec<Document>,
) -> Result<(), CliError> {
    let start = Instant::now();

    let mut configuration = get_configuration()?;
    overrides.apply_to_config(&mut configuration);

    let subscriber = get_subscriber(&configuration, "info".into(), telemetry);
    init_subscriber(subscriber);
    let rt = tokio::runtime::Runtime::new()?;

    rt.block_on(run_server(configuration, start, documents, open))?;

    Ok(())
}
