use std::net::IpAddr;

use gulfi_cli::Cache;

#[derive(Debug, Clone)]
pub struct ApplicationSettings {
    pub port: u16,
    pub host: IpAddr,
    pub cache: Cache,
    pub open: bool,
}

impl ApplicationSettings {
    #[must_use]
    pub fn new(port: u16, host: IpAddr, cache: Cache, open: bool) -> Self {
        Self {
            port,
            host,
            cache,
            open,
        }
    }
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
