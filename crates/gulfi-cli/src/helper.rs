use std::fs::OpenOptions;
use std::io::{self, Seek, Write};
use std::path::Path;

use color_eyre::owo_colors::OwoColorize;
use eyre::{Result, eyre};
use gulfi_common::{Document, Field};

pub const WIDTH: usize = 4;

pub fn delete_document(name: &str) -> Result<()> {
    let path = Path::new("meta.json");

    let mut all_docs: Vec<Document> = if path.exists() {
        let json_str = std::fs::read_to_string(path)?;
        serde_json::from_str(&json_str).unwrap_or_default()
    } else {
        return Err(eyre!("No se ha encontrado un archivo `meta.json`."));
    };

    if let Some(pos) = all_docs
        .iter()
        .position(|doc| doc.name.to_lowercase() == name.to_lowercase())
    {
        all_docs.remove(pos);
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)?;

        Ok(serde_json::to_writer_pretty(file, &all_docs)?)
    } else {
        Err(eyre!("No se encuentra ningun documento con nombre {name}."))
    }
}

pub fn initialize_meta_file() -> Result<()> {
    run_new()
}

pub fn run_new() -> Result<()> {
    let path = Path::new("meta.json");
    if !path.exists() {
        println!(
            "\n{:<WIDTH$}{stage}  No se ha encontrado un archivo `meta.json`. Creando primer documento...",
            "",
            stage = " Gulfi ".bright_white().bold().on_green(),
        );
    }

    let name = prompt_input("Cual sera el nombre del documento?", validate_field_name);

    let mut fields = vec![];
    loop {
        prompt_for_field(&mut fields);
        if !prompt_confirm("¿Añadir otro campo?") {
            break;
        }
    }

    let new_doc = Document { name, fields };

    let mut all_docs: Vec<Document> = if path.exists() {
        let json_str = std::fs::read_to_string(path)?;
        serde_json::from_str(&json_str).unwrap_or_default()
    } else {
        vec![]
    };

    all_docs.push(new_doc);

    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)?;
    file.seek(io::SeekFrom::Start(0))?;

    serde_json::to_writer_pretty(file, &all_docs)?;

    Ok(())
}

fn prompt_input<V>(prompt: &str, validator: V) -> String
where
    V: Fn(&str) -> Result<(), String>,
{
    loop {
        print!(
            "\n{:<WIDTH$}{stage}  {prompt} ",
            "",
            stage = " Builder ".bright_white().bold().on_magenta()
        );
        io::stdout()
            .flush()
            .expect("Tendria que poder hacer flush.");
        let mut buffer = String::new();
        io::stdin()
            .read_line(&mut buffer)
            .expect("Failed to read line");
        let answer = buffer.trim_end().to_owned();
        match validator(&answer) {
            Ok(()) => {
                return answer;
            }
            Err(msg) => {
                println!("Error: {}", msg.bold().bright_red());
            }
        }
    }
}

fn prompt_confirm(msg: &str) -> bool {
    prompt_options(msg, vec!['Y', 'N']) == 'Y'
}

fn prompt_options(msg: &str, opts: Vec<char>) -> char {
    let options = {
        let options_string: Vec<String> = opts.iter().map(|c| format!("{}", c)).collect();
        options_string.join("/")
    };

    let validate_fn = |entry: &str| {
        if entry.len() != 1 {
            return Err(format!("Input invalida. Las opciones son ({})", options));
        }
        let c = entry
            .chars()
            .next()
            .expect("Deberia poder iterarlo.")
            .to_ascii_uppercase();
        if !opts.contains(&c) {
            return Err(format!("Input invalida. Las opciones son ({})", options));
        }
        Ok(())
    };

    let entry = prompt_input(&format!("{} ({})", msg, options), validate_fn);

    entry
        .chars()
        .next()
        .expect("Deberia poder iterarlo.")
        .to_ascii_uppercase()
}

fn prompt_for_field(fields: &mut Vec<Field>) {
    println!();
    let name = prompt_input("Nombre del campo:", validate_field_name);
    let vec_input = prompt_confirm("¿Quieres que sea usado en la busqueda?");
    let unique = prompt_confirm("¿Este campo debería ser único?");

    fields.push(Field {
        name,
        vec_input,
        unique,
    });
}

fn validate_field_name(name: &str) -> Result<(), String> {
    if !name.is_ascii() {
        return Err(String::from(
            "El nombre debe pertenecer a la regex [_a-zA-Z0-9]+",
        ));
    }
    Ok(())
}
