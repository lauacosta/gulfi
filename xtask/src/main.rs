use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::Command;

fn main() {
    let mut args = std::env::args();
    args.next();

    match args.next().as_deref() {
        Some("bundle") => bundle(),
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

fn bundle() {
    let in_ci = std::env::var("CI").is_ok() || std::env::var("GITHUB_ACTIONS").is_ok();

    let bundle = match std::env::var("BUILD_FRONTEND") {
        Ok(val) => {
            println!("cargo:warning=BUILD_FRONTEND set to: {val}");
            val == "true"
        }
        Err(_) => {
            println!(
                "cargo:warning=BUILD_FRONTEND not set, default to: {}",
                !in_ci
            );
            !in_ci
        }
    };

    let ui_dir = Path::new("crates/gulfi_ui/frontend");
    let output_dir = Path::new("crates/gulfi_uifrontend/dist");

    if in_ci && !bundle {
        println!("cargo:warning=Skipping frontend build in CI");

        let placeholder = output_dir.join("placeholder.html");
        let mut file = fs::File::create(&placeholder).expect("Failed to create placeholder file");

        file.write_all(b"<!DOCTYPE html><html><body><h1>Placeholder for CI</h1></body></html>")
            .expect("Failed to write placeholder");

        println!(
            "cargo:warning=Placeholder created for CI at: {}",
            placeholder.display()
        );
        return;
    }

    println!("cargo:warning=Checking pnpm installation...");
    let pnpm_status = Command::new("pnpm")
        .arg("--version")
        .output()
        .expect("Failed to check pnpm installation");

    if !pnpm_status.status.success() {
        panic!(
            "pnpm is not properly installed or accessible. Exit code: {:?}",
            pnpm_status.status.code()
        );
    }

    let pnpm_version = String::from_utf8_lossy(&pnpm_status.stdout);
    println!("cargo:warning=pnpm found, version: {}", pnpm_version.trim());

    println!("cargo:warning=Building frontend with pnpm...");

    let status = Command::new("pnpm")
        .args([
            "build",
            "--outDir",
            output_dir
                .to_str()
                .expect("Output dir path must be valid UTF-8"),
        ])
        .current_dir(ui_dir)
        .status()
        .expect("Failed to execute pnpm build command");

    if !status.success() {
        panic!("Svelte build failed with exit code: {:?}", status.code());
    }

    println!("cargo:warning=UI built successfully at frontend/dist");
}
