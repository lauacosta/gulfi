use clap::crate_version;
use gulfi_common::Document;
use gulfi_server::{ApplicationSettings, startup::run_server};
use std::{net::IpAddr, time::Instant};

use crate::CliError;

#[cfg(debug_assertions)]
use eyre::Report;

#[cfg(debug_assertions)]
use crate::Mode;

#[cfg(debug_assertions)]
use tokio::{process::Command as TokioCommand, try_join};

pub fn start_server(
    interface: IpAddr,
    port: u16,
    open: bool,
    pool_size: usize,
    documents: Vec<Document>,
    #[cfg(debug_assertions)] mode: &Mode,
) -> Result<(), CliError> {
    let start = Instant::now();
    let name = String::from("Gulfi");
    let version = crate_version!().to_owned();

    let configuration = ApplicationSettings::new(name, version, port, interface, open, pool_size);

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
    Ok(())
}
