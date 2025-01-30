use std::{
    fs::File,
    io::BufReader,
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use csv::ReaderBuilder;
use eyre::{Result, eyre};
use futures::StreamExt;
use gulfi_common::{DataSources, Document, HttpError, clean_html, parse_sources};
use gulfi_openai::embed_vec;
use gulfi_ui::Historial;
use rusqlite::{Connection, ToSql, ffi::sqlite3_auto_extension, params_from_iter, types::ValueRef};
use serde_json::Value;
use sqlite_vec::sqlite3_vec_init;
use tracing::{debug, info};
use zerocopy::IntoBytes;

pub async fn sync_vec_tnea(db: &Connection, doc: &Document, base_delay: u64) -> Result<()> {
    let mut statement = db.prepare(&format!("select id, template from {}", doc.name))?;

    let templates: Vec<(u64, String)> = match statement.query_map([], |row| {
        let id: u64 = row.get(0)?;
        let template: String = row.get::<_, String>(1)?;
        Ok((id, template))
    }) {
        Ok(rows) => rows
            .map(|v| v.expect("Deberia tener un template"))
            .collect(),
        Err(err) => return Err(eyre!(err)),
    };

    let chunk_size = 2048;

    info!("Generando embeddings...");

    let client = reqwest::ClientBuilder::new()
        .deflate(true)
        .gzip(true)
        .build()?;

    let jh = templates
        .chunks(chunk_size)
        .enumerate()
        .map(|(proc_id, chunk)| {
            let indices: Vec<u64> = chunk.iter().map(|(id, _)| *id).collect();
            let templates: Vec<String> =
                chunk.iter().map(|(_, template)| template.clone()).collect();
            embed_vec(indices, templates, &client, proc_id, base_delay)
        });

    let stream = futures::stream::iter(jh);

    let start = std::time::Instant::now();
    info!("Insertando nuevas columnas en vec_{}...", doc.name);

    let total_inserted = Arc::new(AtomicUsize::new(0));

    stream.for_each_concurrent(Some(5), |future| {
        let total_inserted = total_inserted.clone();
        async move {
            match future.await {
                Ok(data) => {
                    let mut statement =
                        db.prepare(&format!("insert into vec_{}(row_id, template_embedding) values (?,?)", doc.name)).unwrap();

                    db.execute("BEGIN TRANSACTION", []).expect(
                        "Deberia poder ser convertido a un string compatible con C o hubo un error en SQLite",
                    );
                    let mut insertions = 0;
                    for (id, embedding) in data {
                        insertions += statement.execute(
                            rusqlite::params![id, embedding.as_bytes()],
                        ).expect("Error insertando en vec_tnea");

                    }
                    db.execute("COMMIT", []).expect(
                        "Deberia poder ser convertido a un string compatible con C o hubo un error en SQLite",
                    );

                    total_inserted.fetch_add(insertions, Ordering::Relaxed);
                }
                Err(err) => tracing::error!("Error procesando el chunk: {}", err),
            }
        }
    }).await;

    info!(
        "Insertando nuevos registros en vec_{}... se insertaron {} registros, en {} ms",
        doc.name,
        total_inserted.load(Ordering::Relaxed),
        start.elapsed().as_millis()
    );

    info!("Generando embeddings... listo!");

    Ok(())
}

pub fn sync_fts_tnea(db: &Connection, doc: &Document) {
    let start = std::time::Instant::now();
    let table_name = doc.name.clone();
    info!("Insertando nuevos registros en fts_{}...", table_name);

    let mut fields: String = Default::default();

    for field in &doc.fields {
        if !field.template_member {
            fields.push_str(&format!("{} ,", field.name));
        }
    }

    fields.push_str("template");

    let statement = format!(
        "insert into fts_{table_name} ({fields}) select {fields} from {table_name}; insert into fts_{table_name}(fts_{table_name}) values('optimize');"
    );

    db.execute_batch(&statement)
        .map_err(|err| eyre!(err))
        .expect(
            "Deberia poder ser convertido a un string compatible con C o hubo un error en SQLite",
        );

    info!(
        "Insertando nuevos registros en fts_tnea... listo!. tomó {} ms",
        start.elapsed().as_millis()
    );
}

pub fn init_sqlite() -> Result<String> {
    unsafe {
        sqlite3_auto_extension(Some(std::mem::transmute(sqlite3_vec_init as *const ())));
    }
    let path = std::env::var("DATABASE_URL").map_err(|err| {
        eyre!(
            "La variable de ambiente `DATABASE_URL` no fue encontrada. {}",
            err
        )
    })?;
    Ok(path)
}

pub fn setup_sqlite(db: &rusqlite::Connection, document: &Document) -> Result<()> {
    let table_name = &document.name;
    let (sqlite_version, vec_version): (String, String) =
        db.query_row("select sqlite_version(), vec_version()", [], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })?;

    debug!("sqlite_version={sqlite_version}, vec_version={vec_version}");

    let mut statement = format!(
        "create table if not exists {}_raw (id integer primary key, ",
        table_name
    );
    let length = document.fields.len();
    for (idx, field) in document.fields.iter().enumerate() {
        if idx == length - 1 {
            statement.push_str(&format!("{} text ", field.name));
            break;
        }
        statement.push_str(&format!("{} text, ", field.name));
    }
    statement.push_str(&format!(
        "); create table if not exists {} (id integer primary key,",
        table_name
    ));
    for field in &document.fields {
        if !field.template_member {
            statement.push_str(&format!("{} text, ", field.name));
        }
    }
    statement.push_str("template text ); ");
    statement.push_str(&format!(
        "create virtual table if not exists fts_{} using fts5 ( ",
        table_name
    ));
    for field in &document.fields {
        if !field.template_member {
            statement.push_str(&format!("{},", field.name));
        }
    }
    statement.push_str(&format!(
        "template, content='{}', content_rowid='id'); ",
        table_name
    ));

    statement.push_str(&format!(
        "create virtual table if not exists vec_{} using vec0( row_id integer primary key, template_embedding float[1536]);",
        table_name
    ));

    statement.push_str(
        "
        create table if not exists historial(
            id integer primary key,
            query text not null unique,
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

        ",
    );

    println!("{}", statement);

    db.execute_batch(&statement)
        .map_err(|err| eyre!(err))
        .expect(
            "Deberia poder ser convertido a un string compatible con C o hubo un error en SQLite",
        );

    Ok(())
}

pub fn insert_base_data(db: &rusqlite::Connection, document: &Document) -> Result<()> {
    let table_name = &document.name;
    let num: usize = db.query_row(&format!("select count(*) from {}", table_name), [], |row| {
        row.get(0)
    })?;

    if num != 0 {
        info!("La base de datos contiene {num} registros. Buscando nuevos registros...");
    } else {
        info!("La base de datos se encuentra vacia. Buscando nuevos registros...");
    }

    let start = std::time::Instant::now();
    let inserted = parse_and_insert("./datasources/", db, &document)?;
    info!(
        "Se insertaron {inserted} columnas en {}_raw! en {} ms",
        table_name,
        start.elapsed().as_millis()
    );

    let start = std::time::Instant::now();
    db.execute("BEGIN TRANSACTION", []).expect(
        "Deberia poder ser convertido a un string compatible con C o hubo un error en SQLite",
    );

    let sql_statement = document.generate_template();
    let mut fields = String::new();
    for field in &document.fields {
        if !field.template_member {
            fields.push_str(&format!("{}, ", field.name));
        }
    }

    let statement = format!(
        "insert or ignore into {table_name} ({fields} template) select {fields} {sql_statement} as template from {table_name}_raw"
    );

    let mut statement = db.prepare(&statement)?;

    let inserted = statement
        .execute(rusqlite::params![])
        .map_err(|err| eyre!(err))?;

    info!(
        "Se insertaron {inserted} columnas en {}! en {} ms",
        table_name,
        start.elapsed().as_millis()
    );

    db.execute("COMMIT", []).expect(
        "Deberia poder ser convertido a un string compatible con C o hubo un error en SQLite",
    );

    Ok(())
}

fn compare_records(mut records: Vec<String>, mut headers: Vec<String>) -> eyre::Result<()> {
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

fn parse_and_insert(path: impl AsRef<Path>, db: &Connection, doc: &Document) -> Result<usize> {
    let mut inserted = 0;
    let datasource = parse_sources(path, doc)?;
    info!("Leyendo el directorio: {:?}...", datasource.name);
    for (source, ext) in &datasource.files {
        let mut total_registros = 0;
        info!("Leyendo {source:?}...");

        match ext {
            DataSources::Csv => {
                let mut reader_config = ReaderBuilder::new();
                let mut reader = reader_config
                    .flexible(true)
                    .trim(csv::Trim::All)
                    .has_headers(true)
                    .quote(b'"')
                    .from_path(&source)?;

                let headers: Vec<String> =
                    reader.headers()?.into_iter().map(String::from).collect();

                let named_parameters: Vec<String> =
                    doc.fields.iter().map(|obj| obj.name.clone()).collect();

                compare_records(named_parameters, headers.clone())?;

                let insert_sql = format!(
                    "insert into {}_raw ({}) values ({})",
                    doc.name,
                    headers
                        .iter()
                        .map(|h| format!("\"{}\"", h))
                        .collect::<Vec<_>>()
                        .join(", "),
                    vec!["?"; headers.len()].join(", ")
                );

                let mut stmt = db.prepare(&insert_sql)?;

                db.execute("begin transaction", [])?;

                for result in reader.records() {
                    let record = result?;
                    let values: Vec<Option<String>> = record
                        .iter()
                        .map(|s| Some(clean_html(s.to_string())))
                        .collect();

                    stmt.execute(params_from_iter(values))?;
                    total_registros += 1;
                }

                db.execute("COMMIT", [])?;
            }
            DataSources::Json => {
                let file = File::open(&source)?;
                let reader = BufReader::new(file);
                let data: Vec<Value> = serde_json::from_reader(reader)?;

                if data.is_empty() {
                    continue;
                }

                let headers: Vec<String> = if let Some(first_record) = data.first() {
                    let field_names: Vec<String> = serde_json::to_value(first_record)?
                        .as_object()
                        .unwrap()
                        .keys()
                        .cloned()
                        .collect();

                    field_names
                } else {
                    vec![]
                };

                let named_parameters: Vec<String> =
                    doc.fields.iter().map(|obj| obj.name.clone()).collect();
                compare_records(named_parameters, headers.clone())?;

                let insert_sql = format!(
                    "insert into {}_raw ({}) values ({})",
                    doc.name,
                    headers
                        .iter()
                        .map(|h| format!("\"{}\"", h))
                        .collect::<Vec<_>>()
                        .join(", "),
                    vec!["?"; headers.len()].join(", ")
                );

                let mut stmt = db.prepare(&insert_sql)?;

                db.execute("begin transaction", [])?;

                for record in data {
                    if let Some(map) = record.as_object() {
                        let values: Vec<Option<String>> = headers
                            .iter()
                            .map(|h| {
                                map.get(h)
                                    .and_then(|v| v.as_str().map(|s| clean_html(s.to_string())))
                            })
                            .collect();

                        stmt.execute(params_from_iter(values))?;
                        total_registros += 1;
                    }
                }

                db.execute("COMMIT", [])?;
            }
        };

        info!(
            "Abriendo transacción para insertar nuevos registros en la tabla `{}_raw`.",
            doc.name
        );

        info!(
            "Leyendo {source:?}... listo! - {} nuevos registros",
            total_registros,
        );
        inserted += total_registros;
    }

    Ok(inserted)
}

pub fn update_historial(db: &Connection, query: &str) -> Result<(), HttpError> {
    let updated = db.execute("insert or replace into historial(query) values (?)", [
        query,
    ])?;
    info!("{} registros fueron añadidos al historial!", updated);

    Ok(())
}

pub fn get_historial(db: &Connection) -> Result<Vec<Historial>, HttpError> {
    let mut statement = db.prepare("select id, query from historial order by timestamp desc")?;

    let rows = statement
        .query_map([], |row| {
            let id: u64 = row.get(0).unwrap_or_default();
            let query: String = row.get(1).unwrap_or_default();

            let data = Historial::new(id, query);

            Ok(data)
        })?
        .collect::<Result<Vec<Historial>, _>>()?;

    Ok(rows)
}

pub struct SearchQuery<'a> {
    db: &'a rusqlite::Connection,
    pub stmt_str: String,
    pub bindings: Vec<&'a dyn ToSql>,
}

type QueryResult = (Vec<String>, Vec<Vec<String>>);

impl SearchQuery<'_> {
    pub fn execute(&self) -> Result<QueryResult, HttpError> {
        debug!("{:?}", self.stmt_str);
        let mut statement = self.db.prepare(&self.stmt_str)?;

        let column_names: Vec<String> = statement
            .column_names()
            .into_iter()
            .map(String::from)
            .collect();

        let table = statement
            .query_map(&*self.bindings, |row| {
                let mut data = Vec::new();

                for i in 0..row.as_ref().column_count() {
                    let val = match row.get_ref(i)? {
                        ValueRef::Text(text) => String::from_utf8_lossy(text).into_owned(),
                        ValueRef::Real(real) => format!("{:.3}", -1. * real),
                        ValueRef::Integer(int) => int.to_string(),
                        _ => "Tipo de dato desconocido".to_owned(),
                    };
                    data.push(val);
                }

                Ok(data)
            })?
            .collect::<Result<Vec<Vec<String>>, _>>()?;

        Ok((column_names, table))
    }
}

pub struct SearchQueryBuilder<'a> {
    db: &'a rusqlite::Connection,
    pub stmt_str: String,
    pub bindings: Vec<&'a dyn ToSql>,
}

impl<'a> SearchQueryBuilder<'a> {
    pub fn new(db: &'a rusqlite::Connection, base_stmt: &str) -> Self {
        Self {
            db,
            stmt_str: base_stmt.to_owned(),
            bindings: Vec::new(),
        }
    }

    pub fn add_filter(&mut self, filter: &str, binding: &[&'a dyn ToSql]) {
        self.stmt_str.push_str(filter);
        self.bindings.extend_from_slice(binding);
    }

    pub fn add_bindings(&mut self, binding: &[&'a dyn ToSql]) {
        self.bindings.extend_from_slice(binding);
    }

    pub fn push_str(&mut self, stmt: &str) {
        self.stmt_str.push_str(stmt);
    }

    pub fn build(self) -> SearchQuery<'a> {
        SearchQuery {
            db: self.db,
            stmt_str: self.stmt_str,
            bindings: self.bindings,
        }
    }
}

pub trait QueryMarker {}

#[cfg(test)]
mod tests {
    use super::*;

    // #[test]
    // fn it_works() {
    //     let result = add(2, 2);
    //     assert_eq!(result, 4);
    // }
}
