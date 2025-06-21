use std::{ffi::OsStr, fmt::Write, fs::DirBuilder, io::Write as _, path::PathBuf};

use blake2::{Blake2s256, Digest};
use camino::{Utf8Path, Utf8PathBuf};
use chrono::{DateTime, Utc};
use color_eyre::owo_colors::OwoColorize;
use eyre::Context;
use fs_err::OpenOptions;
use gulfi_common::Document;
use rusqlite::{Connection, Transaction};

pub const MIGRATIONS_PATH: &str = "migrations";
const MIGRATION_TRACKING_TABLE: &str = "schema_migrations";

pub struct Migration {
    pub filename: String,
    pub timestamp: String,
    pub name: String,
    pub content: String,
    pub executed_at: Option<DateTime<Utc>>,
}

pub struct MigrationRunner {
    db_path: Utf8PathBuf,
}

impl MigrationRunner {
    pub fn new<P: AsRef<Utf8Path>>(db_path: P) -> Self {
        Self {
            db_path: db_path.as_ref().to_path_buf(),
        }
    }

    /// Initialize the migration tracking table
    pub fn init(&self) -> eyre::Result<()> {
        let conn = Connection::open(&self.db_path)?;

        conn.execute(&format!("create table if not exists {MIGRATION_TRACKING_TABLE} (filename text primary_key, executed_at datetime default current_timestamp)"),[])?;

        eprintln!("✅ Migration tracking initialized");
        Ok(())
    }

    /// Get all migration files from the migrations directory
    pub fn get_all_migrations(&self) -> eyre::Result<Vec<Migration>> {
        let mut migrations = Vec::new();

        if !Utf8Path::new(MIGRATIONS_PATH).exists() {
            return Ok(migrations);
        }

        for entry in fs_err::read_dir(MIGRATIONS_PATH)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("sql") {
                let filename = path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_string();

                let content = fs_err::read_to_string(&path)?;

                // Format: YYYY_MM_DD_HHMMSS_name.sql
                let parts: Vec<&str> = filename.split('_').collect();
                let timestamp = if parts.len() >= 4 {
                    format!("{}_{}_{}_{}", parts[0], parts[1], parts[2], parts[3])
                } else {
                    "unknown".to_string()
                };

                let name = filename.replace(".sql", "");
                migrations.push(Migration {
                    filename,
                    timestamp,
                    name,
                    content,
                    executed_at: None,
                });
            }
        }

        migrations.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

        Ok(migrations)
    }

    /// Get executed migrations from database
    pub fn get_executed_migrations(&self) -> eyre::Result<Vec<String>> {
        let conn = Connection::open(&self.db_path)?;

        let mut stmt = conn.prepare(&format!(
            "SELECT filename FROM {} ORDER BY executed_at",
            MIGRATION_TRACKING_TABLE
        ))?;

        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;

        let mut executed = Vec::new();
        for row in rows {
            executed.push(row?);
        }

        Ok(executed)
    }

    /// Get pending migrations
    pub fn get_pending_migrations(&self) -> eyre::Result<Vec<Migration>> {
        let all_migrations = self.get_all_migrations()?;
        let executed = self.get_executed_migrations()?;

        let pending: Vec<Migration> = all_migrations
            .into_iter()
            .filter(|m| !executed.contains(&m.filename))
            .collect();

        Ok(pending)
    }

    /// Show migration status
    pub fn status(&self) -> eyre::Result<()> {
        self.init()?;
        let all_migrations = self.get_all_migrations()?;
        let executed = self.get_executed_migrations()?;

        if all_migrations.is_empty() {
            println!("📋 No migrations found");
            return Ok(());
        }

        println!("📋 Migration Status:");
        println!("┌─────────────────────────────────────────┬────────────┐");
        println!("│ Migration                               │ Status     │");
        println!("├─────────────────────────────────────────┼────────────┤");

        for migration in all_migrations {
            let status = if executed.contains(&migration.filename) {
                "✅ Executed"
            } else {
                "⏳ Pending"
            };

            println!("│ {:<39} │ {:<10} │", migration.filename, status);
        }

        println!("└─────────────────────────────────────────┴────────────┘");
        Ok(())
    }

    /// Run pending migrations
    pub fn run(&self, dry_run: bool) -> eyre::Result<()> {
        self.init()?;

        let pending = self.get_pending_migrations()?;

        if pending.is_empty() {
            println!("✅ No pending migrations");
            return Ok(());
        }

        if dry_run {
            println!("🔍 Dry run - would execute {} migrations:", pending.len());
            for migration in &pending {
                println!("  - {}", migration.filename);
            }
            return Ok(());
        }

        println!("🚀 Running {} migrations...", pending.len());

        let mut conn = Connection::open(&self.db_path)?;

        for migration in pending {
            println!("🔄 Running: {}", migration.filename);

            let tx = conn.transaction()?;

            // Execute the migration SQL
            self.execute_migration_sql(&tx, &migration.content)
                .with_context(|| format!("Failed to execute migration: {}", migration.filename))?;

            tx.execute(
                &format!(
                    "insert into {} (filename) values (?1)",
                    MIGRATION_TRACKING_TABLE
                ),
                [&migration.filename],
            )?;

            tx.commit()?;

            println!("✅ Completed: {}", migration.filename);
        }

        println!("🎉 All migrations completed successfully!");
        Ok(())
    }

    /// Execute a single migration's SQL content
    fn execute_migration_sql(&self, tx: &Transaction, sql_content: &str) -> eyre::Result<()> {
        // Parse SQL statements properly, handling triggers and other complex statements
        let statements = self.parse_sql_statements(sql_content);

        for statement in statements {
            let trimmed = statement.trim();
            if !trimmed.is_empty() {
                tx.execute(trimmed, [])?;
            }
        }

        Ok(())
    }

    /// Parse SQL statements, properly handling triggers, procedures, and other complex statements
    fn parse_sql_statements(&self, sql_content: &str) -> Vec<String> {
        let mut statements = Vec::new();
        let mut current_statement = String::new();
        let mut inside_trigger = false;
        let mut inside_begin_end = false;
        let mut begin_count = 0;

        for line in sql_content.lines() {
            let trimmed_line = line.trim().to_lowercase();

            // Track if we're inside a trigger
            if trimmed_line.starts_with("create trigger") {
                inside_trigger = true;
            }

            // Track BEGIN/END blocks
            if trimmed_line.contains("begin") {
                inside_begin_end = true;
                begin_count += 1;
            }

            if trimmed_line.contains("end") && inside_begin_end {
                begin_count -= 1;
                if begin_count == 0 {
                    inside_begin_end = false;
                    inside_trigger = false;
                }
            }

            current_statement.push_str(line);
            current_statement.push('\n');

            // If we hit a semicolon and we're not inside a trigger/BEGIN-END block
            if line.trim().ends_with(';') && !inside_begin_end && !inside_trigger {
                let statement = current_statement.trim().to_string();
                if !statement.is_empty() {
                    // Remove the trailing semicolon for rusqlite
                    let statement = statement
                        .strip_suffix(';')
                        .unwrap_or(&statement)
                        .to_string();
                    statements.push(statement);
                }
                current_statement.clear();
            }
        }

        // Handle any remaining statement
        let remaining = current_statement.trim();
        if !remaining.is_empty() {
            let statement = remaining.strip_suffix(';').unwrap_or(remaining).to_string();
            statements.push(statement);
        }

        statements
    }
    /// Reset all migrations
    pub fn reset(&self) -> eyre::Result<()> {
        let conn = Connection::open(&self.db_path)?;

        conn.execute(
            &format!("drop table if exists {MIGRATION_TRACKING_TABLE}"),
            [],
        )?;

        println!(
            " - Migration tracking reset - {}",
            "all migrations will re-run on next execution"
                .bold()
                .bright_white()
        );
        Ok(())
    }
}

pub fn migrate(db_path: Utf8PathBuf, dry_run: bool) -> eyre::Result<()> {
    let runner = MigrationRunner::new(db_path);

    runner.run(dry_run)
}

pub fn status(db_path: Utf8PathBuf) -> eyre::Result<()> {
    let runner = MigrationRunner::new(db_path);

    runner.status()
}

pub fn fresh(db_path: Utf8PathBuf, dry_run: bool) -> eyre::Result<()> {
    let runner = MigrationRunner::new(db_path);
    runner.reset()?;
    runner.run(dry_run)
}

pub fn create_migration(name: String) {
    let migrations_dir = MIGRATIONS_PATH;
    let timestamp = Utc::now().format("%Y_%m_%d_%H%M%S").to_string();
    let file_name = format!("{timestamp}{name}.sql");
    let migration_file = format!("{migrations_dir}/{file_name}");

    match std::fs::File::create_new(&migration_file) {
        Err(err) => {
            eprintln!(
                "migrations {} already exists!. err: {err}",
                file_name.bright_green().bold()
            );
        }
        Ok(_) => eprintln!(
            "Migration file {} created!",
            file_name.bright_green().bold()
        ),
    }
}

pub fn generate(docs: &[Document]) -> eyre::Result<()> {
    let migrations_dir = MIGRATIONS_PATH;

    DirBuilder::new().recursive(true).create(migrations_dir)?;

    let existing_migrations: Vec<PathBuf> = fs_err::read_dir(migrations_dir)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .collect();

    let base_migrations = "
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

create table if not exists users (
    id integer primary key autoincrement,
    username text not null unique,
    password_hash text not null,
    auth_token text,
    created_at datetime default current_timestamp,
    updated_at datetime default current_timestamp
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

";

    let timestamp = Utc::now().format("%Y_%m_%d_%H%M%S").to_string();
    let file_name = format!("{timestamp}_base_tables.sql",);
    let migration_file = format!("{migrations_dir}/{file_name}");

    if check_for_duplicate(base_migrations, &existing_migrations).is_none() {
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .open(&migration_file)?;

        file.write_all(base_migrations.as_bytes())?;
        eprintln!(
            " - Created migration file {}!",
            file_name.bright_green().bold()
        );
    }

    let timestamp = Utc::now().format("%Y_%m_%d_%H%M%S").to_string();
    let file_name = format!("{timestamp}_docs_tables.sql",);
    let migration_file = format!("{migrations_dir}/{file_name}");

    let mut sql_statement = String::new();
    for doc in docs {
        let _ = writeln!(sql_statement, "{}", doc.as_sql());
    }

    if check_for_duplicate(&sql_statement, &existing_migrations).is_none() {
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .open(&migration_file)?;

        file.write_all(sql_statement.as_bytes())?;
        eprintln!(
            " - Created migration file {}!",
            file_name.bright_green().bold()
        );
    }

    Ok(())
}

fn check_for_duplicate(content: &str, existing_migrations: &[PathBuf]) -> Option<()> {
    let new_hash = calculate_content_hash(content);

    for entry in existing_migrations {
        if entry.extension() == Some(OsStr::new("sql")) {
            let existing_content =
                fs_err::read_to_string(entry.as_path()).expect("Should be able to read the file");
            let existing_hash = calculate_content_hash(&existing_content);

            if new_hash == existing_hash {
                return Some(());
            }
        }
    }

    None
}

fn calculate_content_hash(content: &str) -> String {
    let mut hasher = Blake2s256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}
