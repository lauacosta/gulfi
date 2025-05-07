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
use gulfi_openai::embed_vec_with_progress;
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

pub async fn sync_vec_data(
    db: &Connection,
    doc: &Document,
    base_delay: u64,
) -> Result<(usize, f32)> {
    let doc_name = doc.name.clone();
    let mp = MultiProgress::new();

    eprintln!("Sincronizando tablas VEC en {doc_name}");

    let mut statement = db.prepare(&format!("select id, vec_input from {doc_name}"))?;

    let v_inputs: Vec<(u64, String)> = match statement.query_map([], |row| {
        let id: u64 = row.get(0)?;
        let input: String = row.get::<_, String>(1)?;
        Ok((id, input))
    }) {
        Ok(rows) => rows
            .map(|v| v.expect("Deberia tener una vec_input"))
            .collect(),
        Err(err) => return Err(eyre!(err)),
    };

    eprintln!(
        "{}  Recopilados {} registros para procesamiento",
        "ⓘ".bright_green(),
        v_inputs.len()
    );

    let chunk_size = 2048;
    let chunks = v_inputs.chunks(chunk_size).count();

    let client = reqwest::ClientBuilder::new()
        .deflate(true)
        .gzip(true)
        .build()?;

    let embed_pb = mp.add(ProgressBar::new(chunks as u64));
    let embed_style = ProgressStyle::with_template(
        "   {spinner:.cyan} [{bar:40.green}] {pos}/{len} chunks ({percent}%) [{elapsed_precise}]",
    )
    .expect("Deberia ser un template valido")
    .progress_chars("##-");

    embed_pb.set_style(embed_style);
    embed_pb.enable_steady_tick(Duration::from_millis(100));
    embed_pb.set_message("Generando embeddings");

    let futures_iterator = v_inputs
        .chunks(chunk_size)
        .enumerate()
        .map(|(proc_id, chunk)| {
            let (indices, v_inputs) = chunk.iter().cloned().unzip();

            let (tx, mut rx) = tokio::sync::mpsc::channel::<String>(10);

            let status_pb = mp.insert_before(&embed_pb, ProgressBar::new_spinner());
            status_pb.set_style(
                ProgressStyle::with_template("      {spinner:.yellow} {wide_msg}")
                    .expect("Deberia ser un template valido")
                    .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏ "),
            );
            status_pb.enable_steady_tick(Duration::from_millis(100));
            status_pb.set_message(format!("Procesando chunk {} - preparando...", proc_id + 1));

            tokio::spawn(async move {
                while let Some(msg) = rx.recv().await {
                    if msg.contains("completados") {
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

            embed_vec_with_progress(indices, v_inputs, &client, proc_id, base_delay, tx)
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
                        db.prepare(&format!("insert into vec_{sent_doc_name}(row_id, vec_input_embedding) values (?,?)")).unwrap();

                    db.execute("BEGIN TRANSACTION", []).expect(
                        "Deberia poder ser convertido a un string compatible con C o hubo un error en SQLite",
                    );

                    let mut insertions = 0;
                    for (id, embedding) in data {
                        insertions += statement.execute(
                            rusqlite::params![id, embedding.as_bytes()],
                        ).unwrap_or_else(|_| panic!("Error insertando en vec_{sent_doc_name}"));

                    }
                    db.execute("COMMIT", []).expect(
                        "Deberia poder ser convertido a un string compatible con C o hubo un error en SQLite",
                    );

                     total_inserted.fetch_add(insertions, Ordering::Relaxed);

                     let millis = millis.try_into().unwrap_or_default();
                     acc_time_per_chunk.fetch_add(millis, Ordering::Relaxed);

                    embed_pb.lock().await.inc(1);
                }
                Err(err) => {
                    error!("Error procesando el chunk: {err}");
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

pub fn sync_fts_data(db: &Connection, doc: &Document) -> usize {
    let doc_name = doc.name.clone();

    let pb = ProgressBar::new_spinner();
    let style = ProgressStyle::with_template("{spinner:.green}{wide_msg}")
        .expect("Deberia ser un template valido")
        .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏ ");

    pb.set_style(style);
    pb.enable_steady_tick(Duration::from_millis(100));
    pb.set_message(format!("Sincronizando tablas FTS en {doc_name}..."));

    let field_names = {
        let fields: Vec<String> = doc
            .fields
            .iter()
            .filter(|x| !x.vec_input)
            .map(|x| x.name.clone())
            .collect();

        fields.join(", ")
    };

    let inserted = db
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
        .expect(
            "Deberia poder ser convertido a un string compatible con C o hubo un error en SQLite",
        );

    db.execute(
        &format!("insert into fts_{doc_name}(fts_{doc_name}) values('optimize')"),
        [],
    )
    .map_err(|err| eyre!(err))
    .expect("Deberia poder ser convertido a un string compatible con C o hubo un error en SQLite");

    pb.finish();

    inserted
}

pub fn init_sqlite() -> Result<Connection> {
    unsafe {
        sqlite3_auto_extension(Some(std::mem::transmute::<
            *const (),
            unsafe extern "C" fn(*mut sqlite3, *mut *mut i8, *const sqlite3_api_routines) -> i32,
        >(sqlite3_vec_init as *const ())));
    }

    let path = std::env::var("DATABASE_URL").map_err(|err| {
        eyre!(
            "La variable de ambiente `DATABASE_URL` no fue encontrada. {}",
            err
        )
    })?;

    let db = Connection::open(path)?;

    db.pragma_update(None, "journal_mode", "WAL")?;

    let mode: String = db.pragma_query_value(None, "journal_mode", |row| row.get(0))?;
    debug!("Current journal mode: {}", mode);

    Ok(db)
}

pub fn setup_sqlite(db: &rusqlite::Connection, doc: &Document) -> Result<()> {
    let (sqlite_version, vec_version): (String, String) =
        db.query_row("select sqlite_version(), vec_version()", [], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })?;

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

    db.execute_batch(&statement)
        .map_err(|err| eyre!(err))
        .expect(
            "Deberia poder ser convertido a un string compatible con C o hubo un error en SQLite",
        );

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
                content='{doc_name}', content_rowid='id'
            );

            create virtual table if not exists vec_{doc_name} using vec0(
                row_id integer primary key,
                vec_input_embedding float[{DIMENSION}]
            );
            ",
    );

    debug!(?statement);

    db.execute_batch(&statement)
        .map_err(|err| eyre!(err))
        .expect(
            "Deberia poder ser convertido a un string compatible con C o hubo un error en SQLite",
        );

    // TODO: Decidir si es necesario crear un index por cada campo que pueda ser comparado en un
    // where. Esto porque en la busqueda hago LOWER() sobre esos campos.

    Ok(())
}

pub fn insert_base_data(db: &rusqlite::Connection, doc: &Document) -> Result<()> {
    let doc_name = doc.name.clone();

    let num: usize = db.query_row(&format!("select count(*) from {doc_name}"), [], |row| {
        row.get(0)
    })?;

    if num != 0 {
        eprintln!("📦 La base de datos de '{doc_name}' contiene {num} registros.");
    } else {
        eprintln!("📦 La base de datos de '{doc_name}' está vacía.");
    }

    let start = std::time::Instant::now();
    let db_path = db.path().expect("Deberia poder ver el path a la db");

    eprintln!("📁 Buscando archivos disponibles en \"./datasources/{doc_name}\"...");

    let inserted = parse_and_insert(format!("./datasources/{doc_name}"), db_path, doc)?;
    let elapsed = start.elapsed().as_millis();

    eprintln!(
        "ℹ️ Se insertaron {inserted} registros en {doc_name}_raw! ({elapsed} ms). {}",
        if inserted == 0 {
            "No hubo nuevos registros."
        } else {
            ""
        }
    );

    let start = std::time::Instant::now();
    db.execute("BEGIN TRANSACTION", []).expect(
        "Deberia poder ser convertido a un string compatible con C o hubo un error en SQLite",
    );

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
    let mut statement = db.prepare(&format!(
        "insert or ignore into {doc_name} ({fields_str}, vec_input)
            select {fields_str}, {sql_statement} as vec_input from {doc_name}_raw; "
    ))?;

    let inserted = statement
        .execute(rusqlite::params![])
        .map_err(|err| eyre!(err))?;

    let elapsed = start.elapsed().as_millis();

    eprintln!(
        "ℹ️ Se insertaron {inserted} registros en {doc_name}! ({elapsed} ms). {}",
        if inserted == 0 {
            "No hubo nuevos registros."
        } else {
            ""
        }
    );

    db.execute("COMMIT", []).expect(
        "Deberia poder ser convertido a un string compatible con C o hubo un error en SQLite",
    );

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
        ([], extra) => Err(eyre!("El archivo tiene campos extras: {extra:?}")),

        (missing, []) => Err(eyre!("El archivo no tiene los campos: {missing:?}")),

        (missing, extra) => Err(eyre!(
            "El archivo no tiene los campos: {missing:?} y le sobran los campos: {extra:?}"
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
        .expect("Deberia ser un template valido")
        .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏ ");

    std::thread::scope(|s| {
        let mut jh = vec![];
        for (source, ext) in datasources {
            let inserted = inserted.clone();
            let doc_name = doc_name.clone();
            let multi = multi.clone();
            let style = style.clone();

            let handler: ScopedJoinHandle<eyre::Result<()>> = s.spawn(move || {
                let db = Connection::open(db_path)
                    .expect("Debería ser un path valido a una base de datos sqlite.");

                let pb = multi.add(ProgressBar::new_spinner());
                pb.set_style(style.clone());
                pb.enable_steady_tick(Duration::from_millis(100));
                pb.set_message(format!("Leyendo {source:?}"));

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
                            .filter_map(|row| row.ok())
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
                    "Insertando {} registros de {:?}...",
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
                    "{} Insertado {} registros de {:?}",
                    "✔".bright_green().bold(),
                    total_registros,
                    source.file_name().unwrap_or_default()
                ));
                Ok(())
            });
            jh.push(handler);
        }

        for h in jh {
            if let Err(e) = h.join().expect("El hilo entro en panico.") {
                eprintln!("Ocurrio un error: {:#?}", e);
            }
        }
    });

    let inserted = inserted.load(Ordering::Relaxed);
    Ok(inserted)
}
