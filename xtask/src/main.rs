#![allow(unused)]
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::Command;

use color_eyre::owo_colors::OwoColorize;

#[derive(Debug)]
struct EnvState {
    verbose: bool,
    ci: bool,
    env_variables: HashMap<String, String>,
}

impl EnvState {
    fn new() -> Self {
        let ci = parse_bool_env("CI") || parse_bool_env("GITHUB_ACTIONS");
        let verbose = parse_bool_env("VERBOSE");

        let mut env_variables = HashMap::new();
        let vars_to_capture = ["VERBOSE", "CI", "GITHUB_ACTIONS"];

        for var in vars_to_capture {
            if let Ok(val) = std::env::var(var) {
                env_variables.insert(var.to_string(), val);
            }
        }

        Self {
            verbose,
            ci,
            env_variables,
        }
    }

    fn get(&self, key: &str) -> Option<String> {
        self.env_variables.get(key).cloned()
    }
}

fn parse_bool_env(key: &str) -> bool {
    std::env::var(key)
        .ok()
        .and_then(|val| match val.to_lowercase().as_str() {
            "true" | "1" | "yes" => Some(true),
            "false" | "0" | "no" => Some(false),
            "" => Some(false),
            _ => None,
        })
        .unwrap_or(false)
}

fn main() {
    color_eyre::install().expect("Failed to install color_eyre");

    let mut args = std::env::args();
    args.next();

    let state = EnvState::new();

    println!("{:#?}", state);

    match args.next().as_deref() {
        Some("bundle") => bundle(state),
        Some(cmd) => {
            eprintln!("Unknown xtask command: {}", cmd);
            std::process::exit(1);
        }
        None => {
            eprintln!("No xtask command provided");
            std::process::exit(1);
        }
    }
}

fn bundle(state: EnvState) {
    let ui_source_files_dir = Path::new("frontend");
    let output_dir = Path::new("../crates/gulfi_server/static");

    if state.ci {
        eprintln!("{}", "Skipping frontend build in CI".bold());
        let placeholder = output_dir.join("placeholder.html");
        let mut file = fs::File::create(&placeholder).expect("Failed to create placeholder file");
        file.write_all(b"<!DOCTYPE html><html><body><h1>Placeholder for CI</h1></body></html>")
            .expect("Failed to write placeholder");
        eprintln!(
            "Placeholder created for CI at: {}",
            placeholder.display().green()
        );
        return;
    }

    eprintln!("Calling pnpm CLI:");
    eprintln!("\t- Checking if pnpm is installed...");
    let pnpm_status = Command::new("pnpm")
        .arg("--version")
        .output()
        .expect("Failed to check pnpm installation");

    if !pnpm_status.status.success() {
        panic!(
            "\t- pnpm is not properly installed or accessible. Exit code: {:?}",
            pnpm_status.status.code().red().bold()
        );
    }

    let pnpm_version = String::from_utf8_lossy(&pnpm_status.stdout);
    eprintln!(
        "\t- pnpm found at version: {}",
        pnpm_version.trim().bold().green()
    );

    eprintln!("\t- Building frontend...");
    let status = {
        let mut cmd = Command::new("pnpm");
        cmd.args([
            "build",
            "--outDir",
            output_dir
                .to_str()
                .expect("Output dir path must be valid UTF-8"),
        ])
        .current_dir(ui_source_files_dir);

        if !state.verbose {
            eprintln!("\t- Avoiding printing pnpm output. Use verbose if needed.");
            cmd.stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null());
        }

        cmd.status().expect("Failed to execute pnpm build command")
    };

    if !status.success() {
        panic!(
            "\t- Svelte build failed with exit code: {:?}",
            status.code().bold().red()
        );
    }

    eprintln!(
        "\t- Svelte build successful! static folder located at {:?}",
        output_dir.bold().green()
    );
}
