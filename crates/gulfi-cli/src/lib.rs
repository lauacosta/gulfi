pub mod helper;

use clap::{Parser, Subcommand, ValueEnum, command, crate_version};
use std::net::IpAddr;

#[derive(Parser)]
#[command(version, about,  long_about = None, before_help = format!(r"
 _____       _  __ _ 
|  __ \     | |/ _(_)
| |  \/_   _| | |_ _ 
| | __| | | | |  _| |
| |_\ \ |_| | | | | |
 \____/\__,_|_|_| |_| {}

    @lauacosta/gulfi", crate_version!()
    ))
]
pub struct Cli {
    #[arg(long = "level", default_value = "INFO")]
    pub loglevel: String,

    /// Path a la base de datos sqlite
    #[arg(long = "database-path", default_value = "./gulfi.db")]
    pub db: String,

    #[command(subcommand)]
    command: Option<Command>,
}

impl Cli {
    pub fn command(&self) -> Command {
        self.command.clone().unwrap_or(Command::List)
    }
}

#[derive(Subcommand, Clone, Debug)]
pub enum Command {
    /// Inicia el servidor HTTP y expone la interfaz web
    Serve {
        #[cfg(debug_assertions)]
        #[arg(value_enum)]
        mode: Mode,
        /// Establece la dirección IP.
        #[clap(short = 'I', long, default_value = "127.0.0.1")]
        interface: IpAddr,

        /// Establece el puerto en el cual escuchar.
        #[clap(short = 'P', long, default_value_t = 3000)]
        port: u16,

        /// Automaticamente abre la aplicación en el navegador.
        #[arg(long, default_value = "false")]
        open: bool,
    },
    /// Actualiza la base de datos.
    Sync {
        document: String,

        /// Fuerza la actualización incluso cuando la base de datos no está vacía.
        #[arg(long, default_value = "false")]
        force: bool,

        /// Determina la estrategia para actualizar la base de datos.
        #[arg(value_enum,  default_value_t = SyncStrategy::Fts)]
        sync_strat: SyncStrategy,

        /// Determina la cantidad de tiempo base al hacer backoff en los requests. En millisegundos.
        #[arg(short = 'T', long, default_value_t = 2)]
        base_delay: u64,
    },
    /// Lista todos los documentos disponibles documento.
    List,
    /// Añade un nuevo documento.
    Add,
    /// Borra un documento.
    Delete { document: String },
}

#[cfg(debug_assertions)]
#[derive(Debug, Clone, ValueEnum)]
pub enum Mode {
    Prod,
    Dev,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum SyncStrategy {
    Fts,
    Vector,
    All,
}

#[allow(unused)]
#[derive(Debug, Clone, ValueEnum)]
pub enum Cache {
    Enabled,
    Disabled,
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//
//     // #[test]
//     // fn it_works() {
//     //     let result = add(2, 2);
//     //     assert_eq!(result, 4);
//     // }
// }
//
