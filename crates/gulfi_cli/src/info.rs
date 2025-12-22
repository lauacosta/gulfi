use std::fmt::Write as _;
use std::{
    io::IsTerminal,
    path::{Path, PathBuf},
};

use color_eyre::owo_colors::OwoColorize;
use serde::Serialize;

use crate::MessageFormat;

#[derive(Debug, Serialize)]
pub struct BuildInfo {
    pub version: &'static str,
    pub target: &'static str,
    pub profile: &'static str,
    pub build_timestamp: &'static str,

    pub git_commit: Option<&'static str>,
    pub git_branch: Option<&'static str>,

    pub rust_toolchain: Option<&'static str>,
    pub rustc_version: Option<&'static str>,
    pub cargo_version: Option<&'static str>,
}

impl BuildInfo {
    #![allow(clippy::new_without_default)]
    pub const fn new() -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION"),
            target: env!("BUILD_TARGET"),
            profile: env!("BUILD_PROFILE"),
            build_timestamp: env!("BUILD_TIMESTAMP"),
            git_commit: option_env!("GIT_COMMIT"),
            git_branch: option_env!("GIT_BRANCH"),
            rustc_version: option_env!("RUSTC_VERSION"),
            cargo_version: option_env!("CARGO_VERSION"),
            rust_toolchain: option_env!("RUST_TOOLCHAIN"),
        }
    }

    pub fn display(&self) -> String {
        let mut out = String::new();
        let _ = writeln!(out, "\n{}", "Binary Information:".bold().blue());
        let _ = writeln!(out, "  {}  {}", "Version:".cyan(), self.version);
        let _ = writeln!(
            out,
            "  {}  {}",
            "Build timestamp:".cyan(),
            self.build_timestamp
        );
        let _ = writeln!(out, "  {}  {}", "Target:".cyan(), self.target);
        let _ = writeln!(out, "  {} {}", "Profile:".cyan(), self.profile);

        if let Some(rustc_version) = self.rustc_version {
            let _ = writeln!(out, "  {}  {}", "Rustc:".cyan(), rustc_version);
        }
        if let Some(toolchain) = self.rust_toolchain {
            let _ = writeln!(out, "  {}  {}", "Toolchain:".cyan(), toolchain);
        }
        if let Some(cargo_version) = self.cargo_version {
            let _ = writeln!(out, "  {}  {}", "Cargo:".cyan(), cargo_version);
        }
        if let Some(commit) = self.git_commit {
            let _ = writeln!(out, "  {} {}", "Git commit:".cyan(), commit.yellow());
        }
        if let Some(branch) = self.git_branch {
            let _ = writeln!(out, "  {} {}", "Git branch:".cyan(), branch.yellow());
        }
        out
    }
}
#[derive(Debug, Serialize)]
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

    pub fn display(&self) -> String {
        let mut out = String::new();
        let _ = writeln!(out, "\n{}", "Database Information:".bold().blue());
        let _ = writeln!(out, "  {} {}", "Path:".cyan(), self.path);
        let _ = writeln!(
            out,
            "  {} v{}",
            "sqlite_version:".cyan(),
            self.sqlite_version
        );
        let _ = writeln!(
            out,
            "  {} {}",
            "sqlite-vec_version:".cyan(),
            self.sqlite_vec_version
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
        let _ = writeln!(out, "  {}         {}", "Size:".cyan(), size_str);

        let _ = writeln!(out, "  {}   {}", "Page count:".cyan(), self.page_count);
        let _ = writeln!(out, "  {}    {} bytes", "Page size:".cyan(), self.page_size);
        let _ = writeln!(out, "  {}   {}", "Schema ver:".cyan(), self.schema_version);
        let _ = writeln!(out, "  {}     {}", "Encoding:".cyan(), self.encoding);
        let _ = writeln!(out, "  {} {}", "Journal mode:".cyan(), self.journal_mode);

        let _ = writeln!(
            out,
            "  {}       {} table{}",
            "Tables:".cyan(),
            self.tables.len().to_string().bold(),
            if self.tables.len() == 1 { "" } else { "s" }
        );

        if !self.tables.is_empty() {
            for table in &self.tables {
                let _ = writeln!(out, "    {} {}", "•".green(), table);
            }
        } else {
            let _ = writeln!(out, "    {}", "(no tables)".dimmed());
        }
        out
    }
}

#[derive(Debug, Serialize)]
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

    pub fn display(&self) -> String {
        let mut out = String::new();
        let _ = writeln!(out, "{}", self.build.display());
        if let Some(db) = &self.database {
            let _ = writeln!(out, "{}", db.display());
        } else {
            let _ = writeln!(out, "Database Information: Not available");
        }
        out
    }
}

pub fn info(db_path: &Option<PathBuf>, message_format: MessageFormat) {
    let system_info = SystemInfo::new(db_path);

    let emit_json = matches!(message_format, MessageFormat::Json)
    // Doesnt guarantee ansi_support but I dont really care.
    || !std::io::stdout().is_terminal();

    if emit_json {
        println!(
            "{}",
            serde_json::to_string(&system_info)
                .expect("Failed to deserialize system_info to string")
        );
    } else {
        println!("{}", system_info.display());
    }
}
