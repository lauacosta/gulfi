pub mod pooling;

use std::{
    fmt::Debug,
    fs::File,
    io::BufReader,
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    thread::ScopedJoinHandle,
    time::Duration,
};

use color_eyre::owo_colors::OwoColorize;
use csv::ReaderBuilder;
use eyre::{Result, eyre};
use futures::StreamExt;
use gulfi_common::{DataSources, Document, clean_html, normalize, parse_sources};
use gulfi_openai::OpenAIClient;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use rusqlite::{
    Connection,
    ffi::{sqlite3, sqlite3_api_routines, sqlite3_auto_extension},
};
use serde_json::{Map, Value};
use sqlite_vec::sqlite3_vec_init;
use tokio::sync::Mutex;
use tracing::{debug, error};
use zerocopy::IntoBytes;

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

    let mp = MultiProgress::new();
    eprintln!("Syncing VEC tables in {doc_name}");

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

    eprintln!(
        "{} {} entries to process",
        "ⓘ".bright_green(),
        v_inputs.len()
    );

    let chunks = v_inputs.chunks(chunk_size).count();

    let http_client = reqwest::ClientBuilder::new()
        .deflate(true)
        .gzip(true)
        .build()?;

    let embed_pb = mp.add(ProgressBar::new(chunks as u64));
    let embed_style = ProgressStyle::with_template(
        "   {spinner:.cyan} [{bar:40.green}] {pos}/{len} chunks ({percent}%) [{elapsed_precise}]",
    )
    .expect("Should be an empty template")
    .progress_chars("##-");

    embed_pb.set_style(embed_style);
    embed_pb.enable_steady_tick(Duration::from_millis(100));
    embed_pb.set_message("Generating embeddings");

    let futures_iterator = v_inputs
        .chunks(chunk_size)
        .enumerate()
        .map(|(proc_id, chunk)| {
            let (indices, v_inputs) = chunk.iter().cloned().unzip();

            // TODO: should change to something more robust than a string
            let (tx, mut rx) = tokio::sync::mpsc::channel::<String>(10);

            let status_pb = mp.insert_before(&embed_pb, ProgressBar::new_spinner());
            status_pb.set_style(
                ProgressStyle::with_template("      {spinner:.yellow} {wide_msg}")
                    .expect("Should be an empty template")
                    .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏ "),
            );
            status_pb.enable_steady_tick(Duration::from_millis(100));
            status_pb.set_message(format!("Processing chunk {} - preparing...", proc_id + 1));

            tokio::spawn(async move {
                while let Some(msg) = rx.recv().await {
                    if msg.contains("done") {
                        status_pb.set_message(format!(
                            "{} Chunk {} - {}",
                            "✔".bright_green().bold(),
                            proc_id + 1,
                            msg,
                        ));
                    } else {
                        status_pb.set_message(format!("Chunk {} - {}", proc_id + 1, msg));
                    }
                }
                status_pb.finish();
            });

            client.embed_vec_with_progress(indices, v_inputs, &http_client, proc_id, base_delay, tx)
        });

    let futures_stream = futures::stream::iter(futures_iterator);
    let total_inserted = Arc::new(AtomicUsize::new(0));
    let acc_time_per_chunk = Arc::new(AtomicUsize::new(0));

    futures_stream.for_each_concurrent(Some(6), |future| {
        let total_inserted = total_inserted.clone();
        let acc_time_per_chunk = acc_time_per_chunk.clone();
        let sent_doc_name = doc_name.clone();
        let embed_pb = Mutex::new(embed_pb.clone());

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

                    embed_pb.lock().await.inc(1);
                }
                Err(err) => {
                    error!("Error processing chunk: {err}");
                },
            }
        }
    }).await;

    embed_pb.finish_with_message(format!(
        "{} Embeddings completados",
        "✔".bright_green().bold(),
    ));

    let total = total_inserted.load(Ordering::Relaxed);
    let total_acc_chunks = acc_time_per_chunk.load(Ordering::Relaxed);

    let media = total_acc_chunks as f32 / chunks as f32;

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
    validate_sql_identifier(&doc_name).expect("Should be a safe identifier");

    let pb = ProgressBar::new_spinner();
    let style = ProgressStyle::with_template("{spinner:.green}{wide_msg}")
        .expect("Should be a valid template")
        .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏ ");

    pb.set_style(style);
    pb.enable_steady_tick(Duration::from_millis(100));
    pb.set_message(format!("Syncing FTS tables in {doc_name}..."));

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

    pb.finish();

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
        "ℹ️{inserted} entries inserted in {doc_name}_raw! ({elapsed} ms). {}",
        if inserted == 0 { "No new entries." } else { "" }
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
        "ℹ️{inserted} entries inserted in {doc_name}! ({elapsed} ms). {}",
        if inserted == 0 { "No new entries." } else { "" }
    );

    conn.execute("COMMIT", [])
        .expect("Should be a valid SQL sentence");

    Ok(())
}

fn compare_records(mut records: Vec<String>, mut headers: Vec<String>) -> eyre::Result<()> {
    // FIX: Huh.
    headers.sort();
    records.sort();

    let mut missing_members = vec![];
    let mut extra_members = vec![];

    for h in &headers {
        if !records.contains(h) {
            extra_members.push(h);
        }
    }

    for n in &records {
        if !headers.contains(n) {
            missing_members.push(n);
        }
    }

    match (missing_members.as_slice(), extra_members.as_slice()) {
        ([], []) => Ok(()),
        ([], extra) => Err(eyre!("File has unsupported fields: {extra:?}")),

        (missing, []) => Err(eyre!("File has missing fields: {missing:?}")),

        (missing, extra) => Err(eyre!(
            "File doesn't have fields: {missing:?} but has unsupported fields: {extra:?}"
        )),
    }
}

fn parse_and_insert<T: AsRef<Path> + Debug>(
    path: T,
    db_path: &str,
    doc: &Document,
) -> Result<usize> {
    let inserted = Arc::new(AtomicUsize::new(0));
    let doc_name = doc.name.clone();

    let datasources = parse_sources(&path)?;

    let multi = MultiProgress::new();
    let style = ProgressStyle::with_template("{spinner:.green}   {wide_msg} [{elapsed}]")
        .expect("Should be a valid template")
        .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏ ");

    std::thread::scope(|s| {
        let mut jh = vec![];
        for (source, ext) in datasources {
            let inserted = inserted.clone();
            let doc_name = doc_name.clone();
            let multi = multi.clone();
            let style = style.clone();

            let handler: ScopedJoinHandle<eyre::Result<()>> = s.spawn(move || {
                let db =
                    Connection::open(db_path).expect("Should be a valid path to a sqlite db .");

                let pb = multi.add(ProgressBar::new_spinner());
                pb.set_style(style.clone());
                pb.enable_steady_tick(Duration::from_millis(100));
                pb.set_message(format!("Reading {source:?}"));

                let data = match ext {
                    DataSources::Csv => {
                        let mut reader = ReaderBuilder::new()
                            .flexible(true)
                            .trim(csv::Trim::All)
                            .has_headers(true)
                            .quote(b'"')
                            .from_path(&source)?;

                        let headers: Vec<String> =
                            reader.headers()?.into_iter().map(String::from).collect();

                        let expected_parameters: Vec<String> =
                            doc.fields.iter().map(|obj| obj.name.clone()).collect();

                        compare_records(expected_parameters, headers)?;

                        let records: Vec<Value> = reader
                            .deserialize::<Map<String, Value>>()
                            .filter_map(std::result::Result::ok)
                            .map(Value::Object)
                            .collect();

                        records
                    }
                    DataSources::Json => {
                        let file = File::open(&source)?;
                        let reader = BufReader::new(file);
                        let data: Vec<Value> = serde_json::from_reader(reader)?;

                        let headers: Vec<String> = if let Some(first_record) = data.first() {
                            first_record
                                .as_object()
                                .map(|obj| obj.keys().cloned().collect())
                                .unwrap_or_default()
                        } else {
                            vec![]
                        };

                        let expected_parameters: Vec<String> =
                            doc.fields.iter().map(|obj| obj.name.clone()).collect();

                        compare_records(expected_parameters, headers)?;

                        data
                    }
                };

                let total_registros = data.len();

                pb.set_message(format!(
                    "Inserting {} entries from {:?}...",
                    total_registros,
                    source.file_name().unwrap_or_default()
                ));

                let (fields_str, placeholders_str) = {
                    let fields: Vec<String> = doc.fields.iter().map(|x| x.name.clone()).collect();
                    let fields_str = fields.join(", ");

                    let placeholders: Vec<String> =
                        doc.fields.iter().map(|_| String::from("?")).collect();
                    let placeholders_str = placeholders.join(", ");

                    (fields_str, placeholders_str)
                };
                let expected_fields: Vec<String> =
                    doc.fields.iter().map(|obj| obj.name.clone()).collect();

                let mut statement = db.prepare(&format!(
                    "insert into {doc_name}_raw ({fields_str}) values ({placeholders_str})"
                ))?;

                db.execute("BEGIN TRANSACTION", [])?;

                let input_fields = doc
                    .fields
                    .iter()
                    .map(|x| x.name.clone())
                    .collect::<Vec<String>>();

                for record in data {
                    if let Value::Object(map) = record {
                        let values: Vec<Value> = expected_fields
                            .iter()
                            .map(|field| match map.get(field) {
                                Some(Value::String(s)) => {
                                    if input_fields.contains(field) {
                                        Value::String(clean_html(s.clone()))
                                    } else {
                                        Value::String(normalize(s))
                                    }
                                }
                                Some(other) => other.clone(),
                                None => Value::Null,
                            })
                            .collect();

                        let mut bindings: Vec<&dyn rusqlite::ToSql> = Vec::new();

                        for v in &values {
                            // TODO: Encontrar una manera de mantener las cosas en el stack.
                            match v {
                                Value::String(s) => bindings.push(s as &dyn rusqlite::ToSql),
                                Value::Number(n) if n.is_i64() => {
                                    let val = n.as_i64().expect("Deberia poder castearlo a i64");
                                    let leaked: &'static i64 = Box::leak(Box::new(val));
                                    bindings.push(leaked as &dyn rusqlite::ToSql);
                                }
                                Value::Number(n) if n.is_f64() => {
                                    let val = n.as_f64().expect("Deberia poder castearlo a f64");
                                    let leaked: &'static f64 = Box::leak(Box::new(val));
                                    bindings.push(leaked as &dyn rusqlite::ToSql);
                                }
                                Value::Bool(b) => bindings.push(b as &dyn rusqlite::ToSql),
                                _ => bindings.push(&"" as &dyn rusqlite::ToSql),
                            }
                        }

                        inserted.fetch_add(statement.execute(&bindings[..])?, Ordering::Relaxed);
                    }
                }

                db.execute("COMMIT", [])?;
                pb.finish_with_message(format!(
                    "{} {} entries inserted from {:?}",
                    "✔".bright_green().bold(),
                    total_registros,
                    source.file_name().unwrap_or_default()
                ));
                Ok(())
            });
            jh.push(handler);
        }

        for h in jh {
            if let Err(e) = h.join().expect("Thread panicked.") {
                eprintln!("Something happened. error: {e:#?}");
            }
        }
    });

    let inserted = inserted.load(Ordering::Relaxed);
    Ok(inserted)
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
