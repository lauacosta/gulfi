pub mod embeddings;
pub mod routes;
pub mod startup;

use std::net::IpAddr;

#[derive(Debug, Clone)]
pub struct ApplicationSettings {
    pub port: u16,
    pub host: IpAddr,
    // pub cache: Cache,
    pub open: bool,
}

impl ApplicationSettings {
    #[must_use]
    pub fn new(port: u16, host: IpAddr, open: bool) -> Self {
        Self {
            port,
            host,
            // cache,
            open,
        }
    }
}
