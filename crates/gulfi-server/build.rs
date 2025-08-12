use std::io::Write;
use std::path::Path;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=ui/");

    let in_ci = std::env::var("CI").is_ok() || std::env::var("GITHUB_ACTIONS").is_ok();

    let build_frontend = match std::env::var("BUILD_FRONTEND") {
        Ok(val) => {
            println!("cargo:warning=BUILD_FRONTEND set to: {val}");
            val == "true"
        }
        Err(_) => {
            println!(
                "cargo:warning=BUILD_FRONTEND not set, defaulting to: {}",
                !in_ci
            );
            !in_ci
        }
    };

    let ui_dir = Path::new("ui");
    let output_dir = ui_dir.join("dist");

    if let Err(e) = std::fs::create_dir_all(&output_dir) {
        panic!("Failed to create dist directory: {e}");
    }

    if in_ci && !build_frontend {
        println!("cargo:warning=Skipping frontend build in CI");
        let placeholder = output_dir.join("placeholder.html");

        match std::fs::File::create(&placeholder) {
            Ok(mut file) => {
                if let Err(e) = file.write_all(
                    b"<!DOCTYPE html><html><body><h1>Placeholder for CI</h1></body></html>",
                ) {
                    panic!("Failed to write to placeholder file: {e}");
                }
                println!(
                    "cargo:warning=Placeholder created for CI at: {}",
                    placeholder.display()
                );
            }
            Err(e) => {
                panic!("Failed to create placeholder file: {e}");
            }
        }
        return;
    }

    println!("cargo:warning=Checking pnpm installation...");
    let pnpm_check = match Command::new("pnpm").arg("--version").output() {
        Ok(output) => output,
        Err(e) => {
            panic!(
                "Failed to check pnpm installation: {e}. Make sure pnpm is installed and in PATH."
            );
        }
    };

    if !pnpm_check.status.success() {
        panic!(
            "pnpm is not properly installed or accessible. Exit code: {:?}",
            pnpm_check.status.code()
        );
    }

    let pnpm_version = String::from_utf8_lossy(&pnpm_check.stdout);
    println!("cargo:warning=pnpm found, version: {}", pnpm_version.trim());

    println!("cargo:warning=Building frontend with pnpm...");
    let status = match Command::new("pnpm")
        .args([
            "build",
            "--",
            "--outDir",
            output_dir
                .to_str()
                .expect("Output directory path should be valid UTF-8"),
        ])
        .current_dir(ui_dir)
        .status()
    {
        Ok(status) => status,
        Err(e) => {
            panic!("Failed to execute pnpm build command: {e}");
        }
    };

    if !status.success() {
        panic!("Svelte build failed with exit code: {:?}", status.code());
    }

    if !output_dir.exists() {
        panic!(
            "Build completed successfully, but output directory '{}' was not created!",
            output_dir.display()
        );
    }

    println!(
        "cargo:warning=UI built successfully at: {}",
        output_dir.display()
    );
}
