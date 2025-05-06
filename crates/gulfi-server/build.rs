use std::path::Path;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=ui/");

    let in_ci = std::env::var("CI").is_ok() || std::env::var("GITHUB_ACTIONS").is_ok();
    let build_frontend = match std::env::var("BUILD_FRONTEND") {
        Ok(val) => val == "true",
        Err(_) => !in_ci,
    };

    if in_ci && !build_frontend {
        println!("cargo:warning=Salteando buildear el frontend en CI");
        return;
    }

    let ui_dir = Path::new("ui");
    let output_dir = ui_dir.join("dist");

    if !ui_dir.exists() {
        panic!("Directorio '{}' no existe!", ui_dir.display());
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
