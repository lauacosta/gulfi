use std::process::Command;

fn main() {
    println!(
        "cargo:rustc-env=BUILD_TARGET={}",
        std::env::var("TARGET").unwrap()
    );
    println!(
        "cargo:rustc-env=BUILD_PROFILE={}",
        std::env::var("PROFILE").unwrap()
    );

    let output = Command::new("date")
        .args(["+%Y-%m-%d %H:%M:%S %Z"])
        .output()
        .expect("Failed to get date");
    let build_time = String::from_utf8(output.stdout).unwrap();
    println!("cargo:rustc-env=BUILD_TIMESTAMP={}", build_time.trim());

    if let Ok(output) = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
    {
        if output.status.success() {
            let commit = String::from_utf8(output.stdout).unwrap();
            println!("cargo:rustc-env=GIT_COMMIT={}", commit.trim());
        }
    }

    if let Ok(output) = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
    {
        if output.status.success() {
            let branch = String::from_utf8(output.stdout).unwrap();
            println!("cargo:rustc-env=GIT_BRANCH={}", branch.trim());
        }
    }

    if let Ok(output) = Command::new("rustc").arg("--version").output() {
        if output.status.success() {
            let rustc_version = String::from_utf8(output.stdout).unwrap();
            println!("cargo:rustc-env=RUSTC_VERSION={}", rustc_version.trim());
        }
    }
}
