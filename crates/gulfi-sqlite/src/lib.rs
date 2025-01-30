use std::{
    fs::File,
    io::BufReader,
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use chrono::NaiveDateTime;
use csv::ReaderBuilder;
use eyre::{Result, eyre};
use futures::StreamExt;
use gulfi_common::{
    DataSources, HttpError, Source, TneaData, clean_html, normalize, parse_sources,
};
use gulfi_openai::embed_vec;
use gulfi_ui::{Favoritos, Historial, Resultados};
use rusqlite::{Connection, ToSql, ffi::sqlite3_auto_extension, types::ValueRef};
use sqlite_vec::sqlite3_vec_init;
use tracing::{debug, info};
use zerocopy::IntoBytes;

pub async fn sync_vec_tnea(
    db: &Connection,
    // model: Model,
    base_delay: u64,
) -> Result<()> {
    let mut statement = db.prepare("select id, template from tnea")?;

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
        .map(|(proc_id, chunk)|
        //     match model {
            // Model::OpenAI =>
            {
                let indices: Vec<u64> = chunk.iter().map(|(id, _)| *id).collect();
                let templates: Vec<String> =
                    chunk.iter().map(|(_, template)| template.clone()).collect();
                embed_vec(indices, templates, &client, proc_id, base_delay)
            // }
            // Model::Local => todo!(),
        });

    let stream = futures::stream::iter(jh);

    let start = std::time::Instant::now();
    info!("Insertando nuevas columnas en vec_tnea...");

    let total_inserted = Arc::new(AtomicUsize::new(0));

    stream.for_each_concurrent(Some(5), |future| {
        let total_inserted = total_inserted.clone();
        async move {
            match future.await {
                Ok(data) => {
                    let mut statement =
                        db.prepare("insert into vec_tnea(row_id, template_embedding) values (?,?)").unwrap();

                    db.execute("BEGIN TRANSACTION", []).expect(
                        "Deberia poder ser convertido a un string compatible con C o hubo un error en SQLite",
                    );
                    let mut insertions = 0;
                    for (id, embedding) in data {
                        // tracing::debug!("{id} - {embedding:?}");
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
        "Insertando nuevos registros en vec_tnea... se insertaron {} registros, en {} ms",
        total_inserted.load(Ordering::Relaxed),
        start.elapsed().as_millis()
    );

    info!("Generando embeddings... listo!");

    Ok(())
}

pub fn sync_fts_tnea(db: &Connection) {
    let start = std::time::Instant::now();
    info!("Insertando nuevos registros en fts_tnea...");
    db.execute_batch(
        "
        insert into fts_tnea(rowid, email, provincia, ciudad, edad, sexo, template)
        select rowid, email, provincia, ciudad, edad, sexo, template
        from tnea;

        insert into fts_tnea(fts_tnea) values('optimize');
        ",
    )
    .map_err(|err| eyre!(err))
    .expect("Deberia poder ser convertido a un string compatible con C o hubo un error en SQLite");

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

pub fn setup_sqlite(
    db: &rusqlite::Connection,
    // model: &Model
) -> Result<()> {
    let (sqlite_version, vec_version): (String, String) =
        db.query_row("select sqlite_version(), vec_version()", [], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })?;

    debug!("sqlite_version={sqlite_version}, vec_version={vec_version}");

    let statement = format!(
        "
        create table if not exists tnea_raw(
            id integer primary key,
            email text,
            nombre text,
            sexo text,
            fecha_nacimiento text,
            edad integer not null,
            provincia text,
            ciudad text,
            descripcion text,
            estudios text,
            experiencia text,
            estudios_mas_recientes text
        );

        create table if not exists historial(
            id integer primary key,
            query text not null unique,
            timestamp datetime default current_timestamp
        );

        create table if not exists favoritos (
            id integer primary key,
            nombre text not null unique,
            data text,
            busquedas text,
            timestamp datetime default current_timestamp
        );

        create table if not exists tnea(
            id integer primary key,
            email text unique,
            provincia text,
            ciudad text,
            edad integer not null,
            sexo text,
            template text
        );

        create virtual table if not exists fts_tnea using fts5(
            email, edad, provincia, ciudad, sexo, template,
            content='tnea', content_rowid='id'
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

        {}
        ",
        // match model {
        //     Model::OpenAI => {
        "create virtual table if not exists vec_tnea using vec0(
                    row_id integer primary key,
                    template_embedding float[1536]
                );" // }

                    // Model::Local => {
                    //     // todo!()
                    //     // "create virtual table if not exists vec_tnea using vec0(
                    //     //     row_id integer primary key,
                    //     //     template_embedding float[512]
                    //     // );"
                    // }
                    // }
    );

    db.execute_batch(&statement)
        .map_err(|err| eyre!(err))
        .expect(
            "Deberia poder ser convertido a un string compatible con C o hubo un error en SQLite",
        );

    Ok(())
}

pub fn insert_base_data(db: &rusqlite::Connection, source: &Source) -> Result<()> {
    let num: usize = db.query_row("select count(*) from tnea", [], |row| row.get(0))?;
    if num != 0 {
        info!("La base de datos contiene {num} registros. Buscando nuevos registros...");
    } else {
        info!("La base de datos se encuentra vacia. Buscando nuevos registros...");
    }

    let start = std::time::Instant::now();
    let inserted = parse_and_insert("./datasources/", db, source)?;
    info!(
        "Se insertaron {inserted} columnas en tnea_raw! en {} ms",
        start.elapsed().as_millis()
    );

    let start = std::time::Instant::now();
    db.execute("BEGIN TRANSACTION", []).expect(
        "Deberia poder ser convertido a un string compatible con C o hubo un error en SQLite",
    );

    let sql_statement = source.generate_template();
    let mut statement = db.prepare(&format!(
        "
        insert or ignore into tnea (email, provincia, ciudad, edad, sexo, template)
        select email, provincia, ciudad, edad, sexo, {sql_statement} as template
        from tnea_raw;
        " // where not exists (
          //     select 1 from tnea where tnea.email == tnea_raw.email
          // );
    ))?;

    let inserted = statement
        .execute(rusqlite::params![])
        .map_err(|err| eyre!(err))?;

    info!(
        "Se insertaron {inserted} columnas en tnea! en {} ms",
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

fn parse_and_insert(path: impl AsRef<Path>, db: &Connection, f: &Source) -> Result<usize> {
    let mut inserted = 0;
    let mut statement = db.prepare("select email from tnea_raw")?;
    let emails = statement
        .query_map([], |row| row.get(0))?
        .collect::<Result<Vec<String>, _>>()?;

    let datasources = parse_sources(path)?;
    for (source, ext) in datasources {
        info!("Leyendo {source:?}...");

        let data = match ext {
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
                    f.fields.iter().map(|obj| obj.name.clone()).collect();

                compare_records(named_parameters, headers)?;

                reader
                    .deserialize::<TneaData>()
                    .filter_map(|row| row.ok())
                    .filter(|data| !emails.contains(&data.email))
                    .collect::<Vec<TneaData>>()
            }
            DataSources::Json => {
                let file = File::open(&source)?;
                let reader = BufReader::new(file);
                let data: Vec<TneaData> = serde_json::from_reader(reader)?;

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
                    f.fields.iter().map(|obj| obj.name.clone()).collect();

                compare_records(named_parameters, headers)?;

                data.into_iter()
                    .filter(|row| !emails.contains(&row.email))
                    .collect()
            }
        };
        let total_registros = data.len();

        info!("Abriendo transacción para insertar nuevos registros en la tabla `tnea_raw`.");
        let mut statement = db.prepare(
            "insert into tnea_raw (
            email,
            nombre,
            sexo,
            fecha_nacimiento,
            edad,
            provincia,
            ciudad,
            descripcion,
            estudios,
            estudios_mas_recientes,
            experiencia
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )?;

        db.execute("BEGIN TRANSACTION", [])?;

        for record in data.into_iter() {
            statement.execute((
                &record.email,
                &record.nombre,
                &record.sexo,
                &record.fecha_nacimiento,
                &record.edad,
                normalize(&record.provincia),
                normalize(&record.ciudad),
                clean_html(record.descripcion),
                clean_html(record.estudios),
                clean_html(record.estudios_mas_recientes),
                clean_html(record.experiencia),
            ))?;

            inserted += 1;
        }

        db.execute("COMMIT", [])?;

        info!(
            "Leyendo {source:?}... listo! - {} nuevos registros",
            total_registros,
        );
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

pub fn get_favoritos(db: &Connection) -> Result<Favoritos, HttpError> {
    let mut statement = db.prepare(
        "select id, nombre, data, timestamp, busquedas from favoritos order by timestamp desc",
    )?;

    let rows = statement
        .query_map([], |row| {
            let id: u64 = row.get(0).unwrap_or_default();
            let nombre: String = row.get(1).unwrap_or_default();
            let data: String = row.get(2).unwrap_or_default();
            let timestamp_str: String = row.get(3).unwrap_or_default();
            let bus: String = row.get(4).unwrap_or_default();
            dbg!(&bus);

            let timestamp = NaiveDateTime::parse_from_str(&timestamp_str, "%Y-%m-%d %H:%M:%S")
                .unwrap_or_else(|_| Default::default());

            let busquedas: Vec<String> =
                serde_json::from_str(&bus).expect("Tendria que poder ser serializado");

            let data = Resultados::new(id, nombre, data, timestamp, busquedas);

            Ok(data)
        })?
        .collect::<Result<Vec<Resultados>, _>>()?;

    Ok(Favoritos { favoritos: rows })
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
