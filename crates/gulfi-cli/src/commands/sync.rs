use std::{path::Path, time::Instant};

use color_eyre::owo_colors::OwoColorize;
use gulfi_common::Document;
use gulfi_openai::OpenAIClient;
use gulfi_server::configuration::get_configuration;
use gulfi_sqlite::{create_indexes, spawn_vec_connection, sync_fts_data, sync_vec_data};
use rusqlite::Connection;
use secrecy::ExposeSecret;

use crate::{CliError, ExitOnError, SyncStrategy};

pub fn handle_fts(conn: &Connection, doc: &Document) -> (usize, u128) {
    let start = Instant::now();
    let inserted = sync_fts_data(conn, doc);
    let elapsed = start.elapsed().as_millis();

    (inserted, elapsed)
}

pub fn handle_vector(
    conn: &Connection,
    doc: &Document,
    base_delay: u64,
    chunk_size: usize,
    client: &OpenAIClient,
) -> Result<(usize, f32, u128), CliError> {
    let rt = tokio::runtime::Runtime::new()?;

    let start = Instant::now();
    let (inserted, average) =
        rt.block_on(sync_vec_data(conn, doc, base_delay, chunk_size, client))?;

    let elapsed = start.elapsed().as_millis();

    Ok((inserted, average, elapsed))
}

pub fn handle_update<P: AsRef<Path>>(
    db_path: P,
    doc: &Document,
    strat: &SyncStrategy,
    base_delay: u64,
    chunk_size: usize,
) -> Result<(), CliError> {
    let conn = spawn_vec_connection(db_path)?;
    let configuration = get_configuration()?;
    let client = OpenAIClient::new(
        configuration
            .embedding_provider
            .auth_token
            .expose_secret()
            .to_string(),
        configuration.embedding_provider.endpoint_url,
    );

    match strat {
        SyncStrategy::Fts => {
            let (inserted, elapsed) = handle_fts(&conn, doc);

            eprintln!(
                "{inserted} entries were synced in {} ({elapsed} ms).",
                format!("fts_{}", doc.name).bright_cyan().bold(),
            );
        }
        SyncStrategy::Vector => {
            let (inserted, average, vec_elapsed) =
                handle_vector(&conn, doc, base_delay, chunk_size, &client).or_exit();

            eprintln!(
                "{inserted} entries were synced in {} ({vec_elapsed} ms, average of {average} ms per chunk).",
                format!("vec_{}", doc.name).bright_purple().bold(),
            );
        }
        SyncStrategy::All => {
            let (inserted_fts, fts_elapsed) = handle_fts(&conn, doc);

            let (inserted, average, vec_elapsed) =
                handle_vector(&conn, doc, base_delay, chunk_size, &client).or_exit();

            eprintln!(
                "{inserted_fts} entries were synced in {} ({fts_elapsed} ms).",
                format!("fts_{}", doc.name).bright_cyan().bold(),
            );

            eprintln!(
                "{inserted} entries were synced in {} ({vec_elapsed} ms, average of {average} ms per chunk).",
                format!("vec_{}", doc.name).bright_purple().bold(),
            );
        }
    }

    create_indexes(&conn, doc)?;

    Ok(())
}
