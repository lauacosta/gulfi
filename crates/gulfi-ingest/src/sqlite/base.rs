use std::fmt::Write as _;
use std::io::Write;
use std::sync::atomic::AtomicBool;
use std::{
    fmt::Debug,
    fs::File,
    io::BufReader,
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use color_eyre::owo_colors::OwoColorize;
use csv::ReaderBuilder;
use eyre::{Result, eyre};
use futures::StreamExt;
use gulfi_openai::{OpenAIClient, embedding_message::EmbeddingMessage};
use rusqlite::{
    Connection,
    ffi::{sqlite3, sqlite3_api_routines, sqlite3_auto_extension},
    params_from_iter,
};
use sqlite_vec::sqlite3_vec_init;
use tracing::{debug, error};
use zerocopy::IntoBytes;

use crate::Filetype;
use crate::reader::{Document, parse_sources};

pub const DIMENSION: usize = 1536;
const KEYWORDS: &[&str] = &["SELECT", "DROP", "DELETE", "UPDATE", "INSERT", "TABLE"];

pub async fn sync_vec_data(
    conn: &Connection,
    doc: &Document,
    base_delay: u64,
    chunk_size: usize,
    client: &OpenAIClient,
) -> Result<(usize, f32)> {
    let doc_name = doc.name.clone();
    validate_sql_identifier(&doc_name).expect("Should be a safe identifier");

    let start = std::time::Instant::now();
    println!("Syncing VEC tables in {doc_name}!");
    std::io::stdout()
        .flush()
        .expect("Should be able to flush the pipe");

    let mut statement = conn.prepare_cached(&format!("select id, vec_input from {doc_name}"))?;

    let v_inputs: Vec<(u64, String)> = match statement.query_map([], |row| {
        let id: u64 = row.get(0)?;
        let input: String = row.get::<_, String>(1)?;
        Ok((id, input))
    }) {
        Ok(rows) => rows
            .map(|v| v.expect("Should have a 'vec_input' field"))
            .collect(),
        Err(err) => return Err(eyre!(err)),
    };

    let http_client = reqwest::ClientBuilder::new()
        .deflate(true)
        .gzip(true)
        .build()?;

    let bar_max = 30;
    let chunks = v_inputs.chunks(chunk_size).count();
    let jobs_done = Arc::new(AtomicUsize::new(0));

    let futures_iterator = v_inputs
        .chunks(chunk_size)
        .enumerate()
        .map(|(chunk_id, chunk)| {
            let (indices, v_inputs) = chunk.iter().cloned().unzip();
            let (tx, mut rx) = tokio::sync::mpsc::channel::<EmbeddingMessage>(10);

            let jobs_done = jobs_done.clone();
            tokio::spawn(async move {
                while let Some(msg) = rx.recv().await {
                    if let EmbeddingMessage::Complete { .. } = msg {
                        let jobs_done = 1 + jobs_done.fetch_add(1, Ordering::Relaxed);
                        print!("\r    Progress: ");
                        let bar_current = if chunks == 0 {
                            0
                        } else {
                            bar_max * jobs_done / chunks
                        };

                        for _ in 0..bar_current {
                            print!("#");
                        }
                        for _ in bar_current..bar_max {
                            print!(".");
                        }

                        print!(" ({jobs_done}/{chunks})");
                        std::io::stdout()
                            .flush()
                            .expect("Should be able to flush the pipe");
                    }
                }
            });

            client.embed_vec_with_progress(
                indices,
                v_inputs,
                &http_client,
                chunk_id,
                base_delay,
                tx,
            )
        });

    let futures_stream = futures::stream::iter(futures_iterator);
    let total_inserted = Arc::new(AtomicUsize::new(0));
    let acc_time_per_chunk = Arc::new(AtomicUsize::new(0));

    futures_stream.for_each_concurrent(Some(6), |future| {
        let total_inserted = total_inserted.clone();
        let acc_time_per_chunk = acc_time_per_chunk.clone();
        let sent_doc_name = doc_name.clone();

        async move {
            match future.await {
                Ok((data, millis)) => {

                    let mut statement =
                        conn.prepare(&format!("insert into vec_{sent_doc_name}(row_id, vec_input_embedding) values (?,?)")).expect("Should be able to prepare query");

                    conn.execute("BEGIN TRANSACTION", []).expect(
                        "Should be a valid SQL sentence",
                    );

                    let mut insertions = 0;
                    for (id, embedding) in data {
                        insertions += statement.execute(
                            rusqlite::params![id, embedding.as_bytes()],
                        ).unwrap_or_else(|_| panic!("Error inserting in vec_{sent_doc_name}"));

                    }
                    conn.execute("COMMIT", []).expect(
                    "Should be a valid SQL sentence",
                    );

                     total_inserted.fetch_add(insertions, Ordering::Relaxed);

                     let millis = millis.try_into().unwrap_or_default();
                     acc_time_per_chunk.fetch_add(millis, Ordering::Relaxed);

                }
                Err(err) => {
                    error!("Error processing chunk: {err}");
                },
            }
        }
    }).await;

    let total = total_inserted.load(Ordering::Relaxed);
    let total_acc_chunks = acc_time_per_chunk.load(Ordering::Relaxed);

    let media = total_acc_chunks as f32 / chunks as f32;

    print!("\r    Progress: ");
    for _ in 0..bar_max {
        print!("#");
    }

    print!(" ({chunks}/{chunks})");
    println!();

    println!(
        "{} updated! ({} ms)",
        "VEC tables".bright_purple(),
        start.elapsed().as_millis()
    );
    std::io::stdout()
        .flush()
        .expect("Should be able to flush the pipe");

    Ok((total, media))
}

pub fn create_indexes(conn: &Connection, doc: &Document) -> Result<()> {
    let doc_name = doc.name.clone();
    let queries = vec![format!(
        "CREATE INDEX IF NOT EXISTS idx_{doc_name}_vec_input ON {doc_name}(vec_input) WHERE length(vec_input) > 0"
    )];

    for query in queries {
        conn.execute(&query, [])?;
    }
    Ok(())
}

pub fn sync_fts_data(conn: &Connection, doc: &Document) -> usize {
    let doc_name = doc.name.clone();
    let start = std::time::Instant::now();
    validate_sql_identifier(&doc_name).expect("Should be a safe identifier");

    for field in &doc.fields {
        validate_sql_identifier(&field.name).expect("Should be a safe identifier");
    }

    let field_names = {
        let fields: Vec<String> = doc
            .fields
            .iter()
            .filter(|x| !x.vec_input)
            .map(|x| x.name.clone())
            .collect();

        fields.join(", ")
    };

    let keep_spinning = Arc::new(AtomicBool::new(true));
    let spinner_handle = {
        let keep_spinning = Arc::clone(&keep_spinning);
        let doc_name = doc_name.clone();
        std::thread::spawn(move || {
            let tick_chars = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
            let mut tick_index = 0;

            while keep_spinning.load(Ordering::Relaxed) {
                print!(
                    "\r{} Syncing FTS tables in {}...",
                    tick_chars[tick_index], doc_name
                );
                std::io::stdout()
                    .flush()
                    .expect("Should be able to flush the pipe");

                tick_index = (tick_index + 1) % tick_chars.len();
                std::thread::sleep(Duration::from_millis(100));
            }
        })
    };

    let inserted = conn
        .execute(
            &format!(
                "
            insert into fts_{doc_name}(rowid, {field_names}, vec_input)
            select rowid, {field_names}, vec_input 
            from {doc_name};"
            ),
            [],
        )
        .map_err(|err| eyre!(err))
        .expect("Should be a valid SQL sentence");

    let statements = vec![
        format!("insert into fts_{doc_name}(fts_{doc_name}) values('rebuild')"),
        format!("insert into fts_{doc_name}(fts_{doc_name}) values('optimize')"),
    ];

    for statement in statements {
        conn.execute(&statement, [])
            .expect("Should be a valid SQL sentence");
    }

    keep_spinning.store(false, Ordering::Relaxed);
    spinner_handle.join().expect("Thread shouldn't be dropped");
    println!(
        "\r{} updated! ({} ms)",
        "FTS tables".bright_cyan(),
        start.elapsed().as_millis()
    );
    std::io::stdout()
        .flush()
        .expect("Should be able to flush the pipe");

    inserted
}

pub fn spawn_vec_connection<P: AsRef<Path>>(db_path: P) -> Result<Connection, rusqlite::Error> {
    unsafe {
        sqlite3_auto_extension(Some(std::mem::transmute::<
            *const (),
            unsafe extern "C" fn(*mut sqlite3, *mut *mut i8, *const sqlite3_api_routines) -> i32,
        >(sqlite3_vec_init as *const ())));
    }

    let db = Connection::open(db_path)?;

    db.pragma_update(None, "journal_mode", "WAL")?;

    let mode: String = db.pragma_query_value(None, "journal_mode", |row| row.get(0))?;
    debug!("Current journal mode: {}", mode);

    Ok(db)
}

pub fn setup_sqlite(conn: &rusqlite::Connection, doc: &Document) -> Result<()> {
    let (sqlite_version, vec_version): (String, String) =
        conn.query_row("select sqlite_version(), vec_version()", [], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })?;

    validate_sql_identifier(&doc.name)?;

    debug!("sqlite_version={sqlite_version}, vec_version={vec_version}");
    let statement = "
            create table if not exists historial(
                id integer primary key,
                query text not null unique,
                strategy text,
                doc text,
                peso_fts real,
                peso_semantic real,
                neighbors number,
                timestamp datetime default current_timestamp
            );

            create table if not exists favoritos (
                id integer primary key,
                nombre text not null unique,
                data text,
                doc text,
                busquedas text,
                tipos text,
                timestamp datetime default current_timestamp
            );

            create virtual table if not exists fts_historial using fts5(
                query,
                content='historial', content_rowid='id'
            );

            create trigger if not exists after_insert_historial
                after insert on historial
                begin
                insert into fts_historial(rowid, query) values (new.id, new.query);
            end;

            create trigger if not exists after_update_historial
                after update on historial
                begin
                update fts_historial set query = new.query where rowid = old.id;
            end;

            create trigger if not exists after_delete_historial
                after delete on historial
                begin
                delete from fts_historial where rowid = old.id;
            end;

            "
    .to_owned();

    conn.execute_batch(&statement)
        .map_err(|err| eyre!(err))
        .expect("Should be a valid SQL sentence");

    let doc_name = doc.name.clone();

    let (raw_fields_str, fields_str, field_names) = {
        let fields: Vec<String> = doc
            .fields
            .iter()
            .map(|x| {
                if x.unique {
                    // WARN: Es una buena idea decidir usar conflict ignore?
                    format!("{} text unique on conflict ignore", x.name.clone())
                } else {
                    format!("{} text", x.name.clone())
                }
            })
            .collect();
        let raw_fields_str = fields.join(", ");

        let fields: Vec<String> = doc
            .fields
            .iter()
            .filter(|x| !x.vec_input)
            .map(|x| {
                if x.unique {
                    // WARN: Es una buena idea decidir usar conflict ignore?
                    format!("{} text unique on conflict ignore", x.name.clone())
                } else {
                    format!("{} text", x.name.clone())
                }
            })
            .collect();

        let fields_str = fields.join(", ");

        let fields: Vec<String> = doc
            .fields
            .iter()
            .filter(|x| !x.vec_input)
            .map(|x| x.name.clone())
            .collect();

        let fields_names = fields.join(", ");

        (raw_fields_str, fields_str, fields_names)
    };

    let statement = format!(
        "
            create table if not exists {doc_name}_raw(
                id integer primary key,
                {raw_fields_str}
            );

            create table if not exists {doc_name}(
                id integer primary key,
                {fields_str},
                vec_input text
            );

            create virtual table if not exists fts_{doc_name} using fts5(
                vec_input, {field_names},
                content='{doc_name}',
                content_rowid='id', 
                prefix='2 3 4',
                tokenize='unicode61 remove_diacritics 1'
            );

            create virtual table if not exists vec_{doc_name} using vec0(
                row_id integer primary key,
                vec_input_embedding float[{DIMENSION}]
            );
            ",
    );

    debug!(?statement);

    conn.execute_batch(&statement)
        .map_err(|err| eyre!(err))
        .expect("Should be a valid SQL sentence");

    Ok(())
}

pub fn insert_base_data(conn: &rusqlite::Connection, doc: &Document) -> Result<()> {
    let doc_name = doc.name.clone();

    let num: usize = conn.query_row(&format!("select count(*) from {doc_name}"), [], |row| {
        row.get(0)
    })?;

    if num != 0 {
        eprintln!("📦 Document '{doc_name}' has {num} entries.");
    } else {
        eprintln!("📦 Document '{doc_name}' is empty.");
    }

    let start = std::time::Instant::now();
    let db_path = conn.path().expect("Should be able to access db path");

    eprintln!("📁 Searching files in \"./datasources/{doc_name}\"...");

    let inserted = parse_and_insert(format!("./datasources/{doc_name}"), db_path, doc)?;
    let elapsed = start.elapsed().as_millis();
    eprintln!(
        "Total records processed: {inserted} into {} ({elapsed} ms)",
        format!("{doc_name}_raw").bright_yellow()
    );

    let start = std::time::Instant::now();
    conn.execute("BEGIN TRANSACTION", [])
        .expect("Should be a valid SQL sentence");

    let fields_str = {
        let fields: Vec<String> = doc
            .fields
            .iter()
            .filter(|x| !x.vec_input)
            .map(|x| x.name.clone())
            .collect();

        fields.join(", ")
    };

    let sql_statement = doc.generate_vec_input();
    let mut statement = conn.prepare(&format!(
        "insert or ignore into {doc_name} ({fields_str}, vec_input)
        select {fields_str}, {sql_statement} as vec_input from {doc_name}_raw; "
    ))?;

    let inserted = statement
        .execute(rusqlite::params![])
        .map_err(|err| eyre!(err))?;
    let elapsed = start.elapsed().as_millis();
    eprintln!(
        "Total records processed: {inserted} into {} ({elapsed} ms)",
        doc_name.bright_purple()
    );

    conn.execute("COMMIT", [])
        .expect("Should be a valid SQL sentence");

    Ok(())
}

fn parse_and_insert<T: AsRef<Path> + Debug>(
    path: T,
    db_path: &str,
    doc: &Document,
) -> Result<usize> {
    let doc_name = doc.name.clone();
    let mut total_count = 0;

    let (fields, placeholders) = {
        let mut fields = String::new();
        let mut placeholders = String::new();
        for (i, field) in doc.fields.iter().enumerate() {
            if i > 0 {
                fields.push_str(", ");
                placeholders.push_str(", ");
            }
            write!(fields, "{}", field.name)?;
            write!(placeholders, "?")?;
        }
        (fields, placeholders)
    };
    let sql = format!("INSERT INTO {doc_name}_raw ({fields}) VALUES ({placeholders})");

    let workload = parse_sources(&path)?;
    let bar_max = 30;
    let max_jobs = workload.len();

    let mut files_metadata = Vec::with_capacity(workload.len());
    for (job_counter, (source, ext)) in workload.iter().enumerate() {
        let mut conn = Connection::open(db_path).expect("Should be a valid path to a sqlite db");

        print!("\r    Progress: ");
        let bar_current = if max_jobs == 0 {
            0
        } else {
            bar_max * job_counter / max_jobs
        };

        for _ in 0..bar_current {
            print!("#");
        }
        for _ in bar_current..bar_max {
            print!(".");
        }

        print!(" ({job_counter}/{max_jobs})");
        std::io::stdout().flush()?;

        let start = std::time::Instant::now();
        match ext {
            Filetype::Csv => {
                let mut reader = ReaderBuilder::new()
                    .flexible(true)
                    .trim(csv::Trim::All)
                    .has_headers(true)
                    .quote(b'"')
                    .from_path(source)?;

                let mut headers = reader.headers()?.iter().collect::<Vec<_>>();
                headers.sort_unstable();

                let expected_fields = doc
                    .fields
                    .iter()
                    .map(|f| f.name.as_str())
                    .collect::<Vec<_>>();
                headers.sort_unstable();

                check_headers(&headers, &expected_fields)?;
                let tx = conn.transaction()?;

                {
                    let mut statement = tx.prepare_cached(&sql)?;
                    let mut count = 0;

                    for result in reader.records() {
                        let record = result?;

                        let mut params = Vec::with_capacity(doc.fields.len());

                        for (i, _) in doc.fields.iter().enumerate() {
                            params.push(record.get(i).unwrap_or("").to_string());
                        }

                        statement.execute(params_from_iter(params.iter()))?;

                        count += 1;
                    }
                    total_count += count;
                    files_metadata.push((source, count, start.elapsed()))
                }

                tx.commit()?;
            }
            Filetype::Json => {
                let file = File::open(source)?;
                let reader = BufReader::new(file);
                let data: Vec<serde_json::Value> = serde_json::from_reader(reader)?;

                let mut expected_fields = doc
                    .fields
                    .iter()
                    .map(|f| f.name.as_str())
                    .collect::<Vec<_>>();
                expected_fields.sort_unstable();

                let mut fields = if let Some(first_record) = data.first() {
                    first_record
                        .as_object()
                        .map(|obj| obj.keys().cloned().collect())
                        .unwrap_or_default()
                } else {
                    vec![]
                };
                fields.sort();

                check_headers(&fields, &expected_fields)?;

                let tx = conn.transaction()?;
                {
                    let mut statement = tx.prepare_cached(&sql)?;
                    let mut count = 0;

                    for json_record in &data {
                        let mut params = Vec::with_capacity(doc.fields.len());

                        for field in &doc.fields {
                            let value = json_record
                                .get(&field.name)
                                .map(|v| match v {
                                    serde_json::Value::String(s) => Some(s.clone()),
                                    serde_json::Value::Number(n) => Some(n.to_string()),
                                    serde_json::Value::Bool(b) => Some(b.to_string()),
                                    serde_json::Value::Null => Some(String::new()),
                                    _ => Some(v.to_string()),
                                })
                                .unwrap_or_default();

                            params.push(value);
                        }

                        statement.execute(params_from_iter(params.iter()))?;

                        count += 1;
                    }
                    total_count += count;
                    files_metadata.push((source, count, start.elapsed()))
                }
                tx.commit()?;
            }
        }
    }

    print!("\r    Progress: ");
    for _ in 0..bar_max {
        print!("#");
    }

    print!(" ({max_jobs}/{max_jobs})\n");

    for (path, count, duration) in files_metadata {
        println!(
            "    file {} took {} ms ({} entries)",
            path.to_string_lossy().bright_green(),
            duration.as_millis().bright_purple(),
            count.bright_cyan()
        )
    }

    Ok(total_count)
}

fn validate_sql_identifier(name: &str) -> Result<()> {
    if name.is_empty() || name.len() > 64 {
        return Err(eyre!("Invalid identifier length"));
    }

    match name.chars().next() {
        Some(first) if first.is_alphabetic() => (),
        _ => return Err(eyre!("Identifier must start with a letter")),
    }

    if !name.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Err(eyre!("Identifier contains invalid characters"));
    }

    // Prevent SQL keywords (basic list)
    if KEYWORDS.contains(&name.to_ascii_uppercase().as_str()) {
        return Err(eyre!("Identifier cannot be an SQL keyword"));
    }

    Ok(())
}

fn check_headers<T, U>(actual: &[T], expected: &[U]) -> eyre::Result<()>
where
    T: AsRef<str> + PartialEq + std::fmt::Debug,
    U: AsRef<str> + PartialEq + std::fmt::Debug,
{
    // It is O(n^2) but they are small slices
    let has_extra = actual
        .iter()
        .any(|h| !expected.iter().any(|e| e.as_ref() == h.as_ref()));

    let has_missing = expected
        .iter()
        .any(|h| !actual.iter().any(|a| a.as_ref() == h.as_ref()));

    match (has_missing, has_extra) {
        (false, false) => Ok(()),
        (false, true) => {
            let extra: Vec<_> = actual
                .iter()
                .filter(|h| !expected.iter().any(|e| e.as_ref() == h.as_ref()))
                .map(AsRef::as_ref)
                .collect();
            Err(eyre!("File has unsupported fields: {:?}", extra))
        }

        (true, false) => {
            let missing: Vec<_> = expected
                .iter()
                .filter(|h| !actual.iter().any(|a| a.as_ref() == h.as_ref()))
                .map(AsRef::as_ref)
                .collect();
            Err(eyre!("File has missing fields: {:?}", missing))
        }

        (true, true) => {
            let extra: Vec<_> = actual
                .iter()
                .filter(|h| !expected.iter().any(|e| e.as_ref() == h.as_ref()))
                .map(AsRef::as_ref)
                .collect();
            let missing: Vec<_> = expected
                .iter()
                .filter(|h| !actual.iter().any(|a| a.as_ref() == h.as_ref()))
                .map(AsRef::as_ref)
                .collect();

            Err(eyre!(
                "File doesn't have fields: {:?} but has unsupported fields: {:?}",
                missing,
                extra
            ))
        }
    }
}
