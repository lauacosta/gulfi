use std::{
    fs::{DirBuilder, File, OpenOptions, read_to_string},
    io::{BufRead as _, BufReader, Read, Write},
    path::Path,
};

use crate::CliError;

const CONFIG_TEMPLATE: &str = r#"
# Application Configuration
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
tracer_provider:
    service_name: my-app
    protocol: HttpBinary
    api_key: "secret-api-key"
    endpoint: "endpoint"
"#;

pub fn create_config_template() -> Result<(), CliError> {
    let config_path = "configuration/config.yml";
    let config_entry = "/configuration";

    if Path::new(config_path).exists() {
        let contents = read_to_string(config_path).expect("File should be present");

        if !contents.trim().is_empty() {
            eprintln!("Config file already exists and it's not empty.");
            return Ok(());
        }
    }

    DirBuilder::new().recursive(true).create("configuration")?;

    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(config_path)?;
    file.write_all(CONFIG_TEMPLATE.as_bytes())?;

    if Path::new(".git").exists() {
        if Path::new(".gitignore").exists() {
            let gitignore_file = File::open(".gitignore")?;
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
                println!("ℹ️  /configuration already set in .gitignore");
            } else {
                let mut gitignore_file = OpenOptions::new().append(true).open(".gitignore")?;
                let mut contents = String::new();

                File::open(".gitignore")?.read_to_string(&mut contents)?;
                if !contents.ends_with('\n') && !contents.is_empty() {
                    writeln!(gitignore_file)?;
                }

                writeln!(gitignore_file, "{config_entry}")?;
                println!("✅ Added /configuration .gitignore");
            }
        } else {
            let mut gitignore_file = File::create(".gitignore")?;
            writeln!(gitignore_file, "{config_entry}")?;
            println!("✅ Created .gitignore with /configuration entry");
        }
    } else {
        println!("ℹ️  Not a Git repository - skipping .gitignore creation");
    }

    println!("✅ config.yml created successfully!");
    Ok(())
}
