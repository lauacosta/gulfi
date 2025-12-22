use std::path::{Path, PathBuf};

use color_eyre::owo_colors::OwoColorize;

#[derive(Debug)]
pub struct BuildInfo {
    pub version: &'static str,
    pub target: &'static str,
    pub profile: &'static str,
    pub build_date: &'static str,
    pub git_commit: Option<&'static str>,
    pub git_branch: Option<&'static str>,
    pub rustc_version: &'static str,
}

impl BuildInfo {
    pub const fn new() -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION"),
            target: env!("BUILD_TARGET"),
            profile: env!("BUILD_PROFILE"),
            build_date: env!("BUILD_TIMESTAMP"),
            git_commit: option_env!("GIT_COMMIT"),
            git_branch: option_env!("GIT_BRANCH"),
            rustc_version: env!("RUSTC_VERSION"),
        }
    }

    pub fn print(&self) {
        println!("\n{}", "Binary Information:".bold().blue());
        println!("  {}  {}", "Version:".cyan(), self.version);
        println!("  {}  {}", "Built:".cyan(), self.build_date);
        println!("  {}  {}", "Target:".cyan(), self.target);
        println!("  {} {}", "Profile:".cyan(), self.profile);
        println!("  {}  {}", "Rustc:".cyan(), self.rustc_version);
        if let Some(commit) = self.git_commit {
            println!("  {} {}", "Git commit:".cyan(), commit.yellow());
        }
        if let Some(branch) = self.git_branch {
            println!("  {} {}", "Git branch:".cyan(), branch.yellow());
        }
    }
}
#[derive(Debug)]
struct DatabaseInfo {
    path: String,
    sqlite_version: String,
    sqlite_vec_version: String,
    size_bytes: u64,
    page_count: i64,
    page_size: i64,
    schema_version: i64,
    tables: Vec<String>,
    encoding: String,
    journal_mode: String,
}

impl DatabaseInfo {
    fn from_path(db_path: impl AsRef<Path>) -> rusqlite::Result<Self> {
        let conn = gulfi_ingest::spawn_vec_connection(&db_path)?;
        let path = db_path.as_ref().display().to_string();

        let size_bytes = std::fs::metadata(db_path).map(|m| m.len()).unwrap_or(0);

        let (sqlite_version, vec_version): (String, String) =
            conn.query_row("select sqlite_version(), vec_version()", [], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })?;

        let page_count: i64 = conn.query_row("PRAGMA page_count", [], |row| row.get(0))?;
        let page_size: i64 = conn.query_row("PRAGMA page_size", [], |row| row.get(0))?;
        let schema_version: i64 = conn.query_row("PRAGMA schema_version", [], |row| row.get(0))?;
        let encoding: String = conn.query_row("PRAGMA encoding", [], |row| row.get(0))?;
        let journal_mode: String = conn.query_row("PRAGMA journal_mode", [], |row| row.get(0))?;

        let mut stmt = conn.prepare(
            "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name"
        )?;
        let tables: Vec<String> = stmt
            .query_map([], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self {
            path,
            size_bytes,
            sqlite_version,
            sqlite_vec_version: vec_version,
            page_count,
            page_size,
            schema_version,
            tables,
            encoding,
            journal_mode,
        })
    }

    pub fn print(&self) {
        println!("\n{}", "Database Information:".bold().blue());
        println!("  {} {}", "Path:".cyan(), self.path.bright_white());
        println!(
            "  {} v{}",
            "sqlite_version:".cyan(),
            self.sqlite_version.bright_white()
        );
        println!(
            "  {} {}",
            "sqlite-vec_version:".cyan(),
            self.sqlite_vec_version.bright_white()
        );

        let size_mb = self.size_bytes as f64 / 1_048_576.0;
        let size_str = if size_mb < 0.01 {
            format!(
                "{} bytes ({:.2} KB)",
                self.size_bytes,
                self.size_bytes as f64 / 1024.0
            )
        } else {
            format!("{} bytes ({:.2} MB)", self.size_bytes, size_mb)
        };
        println!("  {}         {}", "Size:".cyan(), size_str.bright_white());

        println!(
            "  {}   {}",
            "Page count:".cyan(),
            self.page_count.to_string().bright_white()
        );
        println!(
            "  {}    {} bytes",
            "Page size:".cyan(),
            self.page_size.to_string().bright_white()
        );
        println!(
            "  {}   {}",
            "Schema ver:".cyan(),
            self.schema_version.to_string().bright_white()
        );
        println!(
            "  {}     {}",
            "Encoding:".cyan(),
            self.encoding.bright_white()
        );
        println!(
            "  {} {}",
            "Journal mode:".cyan(),
            self.journal_mode.bright_white()
        );

        println!(
            "  {}       {} table{}",
            "Tables:".cyan(),
            self.tables.len().to_string().bright_white().bold(),
            if self.tables.len() == 1 { "" } else { "s" }
        );

        if !self.tables.is_empty() {
            for table in &self.tables {
                println!("    {} {}", "•".green(), table.bright_white());
            }
        } else {
            println!("    {}", "(no tables)".dimmed());
        }
    }
}

#[derive(Debug)]
struct SystemInfo {
    build: BuildInfo,
    database: Option<DatabaseInfo>,
}

impl SystemInfo {
    pub fn new(db_path: &Option<PathBuf>) -> Self {
        let build = BuildInfo::new();
        let database = match db_path {
            Some(path) => DatabaseInfo::from_path(path).ok(),
            None => None,
        };

        Self { build, database }
    }

    pub fn print(&self) {
        self.build.print();
        if let Some(db) = &self.database {
            db.print();
        } else {
            println!("Database Information: Not available");
        }
    }
}

pub fn info(db_path: &Option<PathBuf>) {
    SystemInfo::new(db_path).print();
}
