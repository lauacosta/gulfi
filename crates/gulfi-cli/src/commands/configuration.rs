use std::{
    fs::{DirBuilder, File, OpenOptions},
    io::{Read, Write},
    path::Path,
};

use config::ConfigError;

use crate::CliError;

pub fn create_config_template() -> Result<(), CliError> {
    let config_content = r#"# Application Configuration
app_settings:
  name: "MyApp"
  port: "3000"
  host: "127.0.0.1"
  meta_file_path: "./meta.json"
embedding_provider:
  endpoint_url: "https://api.openai.com/v1/embeddings"
  auth_token: "your-secret-token-here"
db_settings:
  pool_size: "10"
  db_path: "./gulfi.db"
"#;

    let config_path = "configuration/config.yml";

    if Path::new(config_path).exists() {
        let mut existing_file = File::open(config_path)?;
        let mut contents = String::new();
        existing_file.read_to_string(&mut contents)?;

        if !contents.trim().is_empty() {
            return Err(ConfigError::Message(
                "Config file already exists and is not empty. Please remove or rename the existing config.yml file.".to_string()
            ).into());
        }
    }

    DirBuilder::new().recursive(true).create("configuration")?;

    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(config_path)?;

    file.write_all(config_content.as_bytes())?;
    println!("✅ config.yml created successfully!");
    Ok(())
}
