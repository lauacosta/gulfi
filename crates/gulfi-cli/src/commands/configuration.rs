use std::{
    fs::{DirBuilder, File, OpenOptions},
    io::{BufRead as _, BufReader, Read, Write},
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
    let gitignore_path = ".gitignore";
    let config_entry = "/configuration";

    let is_git_repo = Path::new(".git").exists();

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

    if is_git_repo {
        if Path::new(gitignore_path).exists() {
            let gitignore_file = File::open(gitignore_path)?;
            let reader = BufReader::new(gitignore_file);
            let mut found_config_entry = false;

            for line in reader.lines() {
                let line = line?;
                if line.trim() == config_entry {
                    found_config_entry = true;
                    break;
                }
            }

            if found_config_entry {
                println!("ℹ️  /configuration already exists in .gitignore");
            } else {
                let mut gitignore_file = OpenOptions::new().append(true).open(gitignore_path)?;

                let mut contents = String::new();
                File::open(gitignore_path)?.read_to_string(&mut contents)?;
                if !contents.ends_with('\n') && !contents.is_empty() {
                    writeln!(gitignore_file)?;
                }

                writeln!(gitignore_file, "{config_entry}")?;
                println!("✅ Added /configuration to existing .gitignore");
            }
        } else {
            let mut gitignore_file = File::create(gitignore_path)?;
            writeln!(gitignore_file, "{config_entry}")?;
            println!("✅ Created .gitignore with /configuration entry");
        }
    } else {
        println!("ℹ️  Not a Git repository - skipping .gitignore creation");
    }

    println!("✅ config.yml created successfully!");
    Ok(())
}
