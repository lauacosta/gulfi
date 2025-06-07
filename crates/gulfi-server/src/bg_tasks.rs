use rusqlite::{Connection, params};
use std::{
    path::Path,
    sync::{Arc, Mutex},
};
use tokio::sync::mpsc;
use tracing::{info_span, instrument};

use crate::search::SearchStrategy;

#[derive(Debug)]
pub enum WriteJob {
    History {
        query: String,
        doc: String,
        strategy: SearchStrategy,
        peso_fts: f32,
        peso_semantic: f32,
        k_neighbors: u64,
    },
    Cache {
        query: String,
        result_json: String,
        expires_at: i64,
    },
}

#[instrument(name = "bg_task", fields(db_path, job))]
pub fn spawn_writer_task<P: AsRef<Path>>(
    db_path: P,
) -> eyre::Result<mpsc::UnboundedSender<WriteJob>> {
    let conn = Connection::open(db_path)?;
    let (tx, mut rx) = mpsc::unbounded_channel();

    let conn = Arc::new(Mutex::new(conn));

    tokio::spawn({
        async move {
            while let Some(job) = rx.recv().await {
                let conn = conn.clone();

                let res = tokio::task::spawn_blocking(move || {
                    let conn = conn.lock().expect("Lock should be obtainable");
                    match job {
                        WriteJob::History {
                            query,
                            doc,
                            strategy,
                            peso_fts,
                            peso_semantic,
                            k_neighbors,
                        } => {
                            let insert_span = info_span!("bg_task.history");
                            let _guard = insert_span.enter();
                            let mut stmt = conn.prepare_cached(
                                "insert or replace into historial(query, strategy, doc, peso_fts, peso_semantic, neighbors) values (?,?,?,?,?,?)")?;

                            stmt.execute(params![
                                query,
                                strategy,
                                doc,
                                peso_fts,
                                peso_semantic,
                                k_neighbors
                            ])
                        }
                        WriteJob::Cache {
                            query: _,
                            result_json: _,
                            expires_at: _,
                        } => todo!(),
                    }
                }).await;

                if let Err(e) = res {
                    eprintln!("[writer task] Write failed: {e:?}");
                }
            }
        }
    });

    Ok(tx)
}
