use secrecy::SecretString;
use serde::Deserialize;
use serde_aux::field_attributes::deserialize_number_from_string;
use std::{net::IpAddr, path::PathBuf};

#[derive(Deserialize, Debug, Clone)]
pub struct Settings {
    pub app_settings: ApplicationSettings,
    pub embedding_provider: EmbeddingProviderSettings,
    pub db_settings: DatabaseSettings,
}

#[derive(Deserialize, Debug, Clone)]
pub struct DatabaseSettings {
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub pool_size: usize,
    pub db_path: PathBuf,
}

#[derive(Deserialize, Debug, Clone)]
pub struct EmbeddingProviderSettings {
    pub endpoint_url: String,
    pub auth_token: SecretString,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ApplicationSettings {
    pub name: String,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
    pub host: IpAddr,
    pub meta_file_path: PathBuf,
}

impl ApplicationSettings {
    #[must_use]
    pub fn new(name: String, port: u16, host: IpAddr, meta_file_path: PathBuf) -> Self {
        Self {
            name,
            port,
            host,
            meta_file_path,
        }
    }
}

pub fn get_configuration() -> Result<Settings, config::ConfigError> {
    let base_path = std::env::current_dir().expect("Failed to determine the current directory");
    let config_dir = base_path.join("configuration");
    // let env: Environment = std::env::var("APP_ENVIRONMENT")
    //     .unwrap_or_else(|_| "local".into())
    //     .try_into()
    //     .expect("Failed to parse APP_ENVIRONMENT.");

    let settings = config::Config::builder()
        .add_source(config::File::from(config_dir.join("config")).required(true))
        // .add_source(config::File::from(config_dir.join(env.as_str())).required(true))
        // .add_source(config::Environment::with_prefix("app").separator("__"))
        .build()
        .expect("Failed to read config file");

    let settings: Settings = settings
        .try_deserialize()
        .expect("Failed to deserialize the config into Settings struct");

    Ok(settings)
}
pub enum Environment {
    Local,
    Production,
}

impl Environment {
    pub fn as_str(&self) -> &'static str {
        match self {
            Environment::Local => "local",
            Environment::Production => "production",
        }
    }
}

impl TryFrom<String> for Environment {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "local" => Ok(Self::Local),
            "production" => Ok(Self::Production),
            other => Err(format!(
                "{other} is not a supported environment. Use either 'local' or 'production'"
            )),
        }
    }
}
