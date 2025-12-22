use std::{path::Path, time::Instant};

use color_eyre::owo_colors::OwoColorize;
use gulfi_ingest::{
    Document, SyncStatistics, create_indexes, spawn_vec_connection, update_fts_data,
    update_vec_data,
};
use gulfi_openai::OpenAIClient;
use gulfi_server::configuration::get_configuration;
use rusqlite::Connection;
use secrecy::ExposeSecret;

use crate::{CliError, ExitOnError, UpdateStrategy};

pub fn handle_fts(conn: &Connection, doc: &Document) -> (usize, u128) {
    let start = Instant::now();
    let inserted = update_fts_data(conn, doc);
    let elapsed = start.elapsed().as_millis();

    (inserted, elapsed)
}

pub fn handle_vector(
    conn: &Connection,
    doc: &Document,
    base_delay: u64,
    chunk_size: usize,
    client: &OpenAIClient,
) -> Result<SyncStatistics, CliError> {
    Ok(update_vec_data(conn, doc, base_delay, chunk_size, client)?)
}

pub fn handle<P: AsRef<Path>>(
    db_path: P,
    doc: &Document,
    strat: &UpdateStrategy,
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
        UpdateStrategy::Fts => {
            let (inserted, elapsed) = handle_fts(&conn, doc);

            eprintln!(
                "{inserted} entries were synced in {} ({elapsed} ms).",
                format!("fts_{}", doc.name).bright_cyan().bold(),
            );
        }
        UpdateStrategy::Vector => {
            let stats = handle_vector(&conn, doc, base_delay, chunk_size, &client).or_exit();

            eprintln!(
                "{} entries were synced in {} ({} ms, average of {} ms per chunk).",
                stats.total_inserted,
                stats.time_elapsed,
                stats.average,
                format!("vec_{}", doc.name).bright_purple().bold(),
            );
        }
        UpdateStrategy::All => {
            let (inserted_fts, fts_elapsed) = handle_fts(&conn, doc);
            let stats = handle_vector(&conn, doc, base_delay, chunk_size, &client).or_exit();

            eprintln!(
                "{inserted_fts} entries were synced in {} ({fts_elapsed} ms).",
                format!("fts_{}", doc.name).bright_cyan().bold(),
            );

            eprintln!(
                "{} entries were synced in {} ({} ms, average of {} ms per chunk).",
                stats.total_inserted,
                stats.time_elapsed,
                stats.average,
                format!("vec_{}", doc.name).bright_purple().bold(),
            );
        }
    }

    create_indexes(&conn, doc)?;

    Ok(())
}
