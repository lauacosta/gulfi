use std::io::Write;
use std::path::Path;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=ui/");

    let in_ci = std::env::var("CI").is_ok() || std::env::var("GITHUB_ACTIONS").is_ok();
    let build_frontend = match std::env::var("BUILD_FRONTEND") {
        Ok(val) => val == "true",
        Err(_) => !in_ci,
    };

    let ui_dir = Path::new("ui");
    let output_dir = ui_dir.join("dist");
    std::fs::create_dir_all(&output_dir).expect("Fallo al crear el directorio 'dist'");

    if in_ci && !build_frontend {
        println!("cargo:warning=Salteando buildear el frontend en CI");

        let placeholder = output_dir.join("placeholder.html");
        let mut file = std::fs::File::create(placeholder).expect("Fallo al crear placeholder");
        file.write_all(b"<!DOCTYPE html><html><body><h1>Placeholder for CI</h1></body></html>")
            .expect("Fallo al escribir en el placeholder");

        println!("cargo:warning=Placeholder creador para CI");

        return;
    }

    let pnpm_check = Command::new("pnpm")
        .arg("--version")
        .output()
        .expect("Fallo al chequear la instalacion de pnpm");

    if !pnpm_check.status.success() {
        panic!("pnpm no está instalado.");
    }

    println!(
        "pnpm encontrado: {}",
        String::from_utf8_lossy(&pnpm_check.stdout)
    );

    let status = Command::new("pnpm")
        .args([
            "build",
            "--",
            "--outDir",
            output_dir
                .to_str()
                .expect("El directorio tendria que existir"),
        ])
        .current_dir(ui_dir)
        .status()
        .expect("El proceso de build falló.");

    if !status.success() {
        panic!("Svelte build falló!");
    }

    if !output_dir.exists() {
        panic!(
            "Build exitosa, pero '{}' no fue creado!",
            output_dir.display()
        );
    }

    println!("La UI se construyó exitosamente.");
}
