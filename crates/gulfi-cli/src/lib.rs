use std::net::IpAddr;

use clap::{Parser, Subcommand, ValueEnum, command};

#[derive(Parser)]
#[command(version, about, long_about = None, before_help = r"
 _____       _  __ _ 
|  __ \     | |/ _(_)
| |  \/_   _| | |_ _ 
| | __| | | | |  _| |
| |_\ \ |_| | | | | |
 \____/\__,_|_|_| |_| v1.0.0

    @lauacosta/gulfi")]
pub struct Cli {
    #[arg(long = "log-level", default_value = "INFO")]
    pub loglevel: String,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Inicia el cliente web.
    Serve {
        /// Establece la dirección IP.
        #[clap(short = 'I', long, default_value = "127.0.0.1")]
        interface: IpAddr,

        /// Establece el puerto en el cual escuchar.
        #[clap(short = 'P', long, default_value_t = 3000)]
        port: u16,

        #[arg(value_enum, short = 'C', long, default_value_t = Cache::Disabled)]
        cache: Cache,

        /// Automaticamente abre la aplicación en el navegador
        #[arg(long, default_value = "false")]
        open: bool,
    },
    /// Actualiza las bases de datos
    Sync {
        /// Fuerza la actualización incluso cuando la base de datos no está vacía.
        #[arg(long, default_value = "false")]
        clean_slate: bool,

        /// Determina la estrategia para actualizar la base de datos.
        #[arg(value_enum, short = 'S', long, default_value_t = SyncStrategy::Fts)]
        sync_strat: SyncStrategy,

        /// Determina la cantidad de tiempo base al hacer backoff en los requests. En millisegundos.
        #[arg(short = 'T', long, default_value_t = 2)]
        base_delay: u64,
    },
}

#[derive(Clone, ValueEnum)]
pub enum SyncStrategy {
    Fts,
    Vector,
    All,
}

#[derive(Clone, ValueEnum)]
pub enum Model {
    OpenAI,
    Local,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum Cache {
    Enabled,
    Disabled,
}

#[cfg(test)]
mod tests {
    use super::*;

    // #[test]
    // fn it_works() {
    //     let result = add(2, 2);
    //     assert_eq!(result, 4);
    // }
}
