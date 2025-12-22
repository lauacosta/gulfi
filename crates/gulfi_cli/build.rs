use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs");

    println!(
        "cargo:rustc-env=BUILD_TARGET={}",
        std::env::var("TARGET").expect("Couldn't find TARGET var")
    );
    println!(
        "cargo:rustc-env=BUILD_PROFILE={}",
        std::env::var("PROFILE").expect("couldn't find PROFILE var")
    );

    let build_timestamp = if let Ok(epoch) = std::env::var("SOURCE_DATE_EPOCH") {
        epoch
    } else {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Couldn't see the duration since the unix epoch")
            .as_secs()
            .to_string()
    };
    println!("cargo:rustc-env=BUILD_TIMESTAMP={}", build_timestamp);

    if let Ok(output) = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        && output.status.success()
    {
        let commit =
            String::from_utf8(output.stdout).expect("Couldn't parse the commit id from utf8");
        println!("cargo:rustc-env=GIT_COMMIT={}", commit.trim());
    }

    if let Ok(output) = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        && output.status.success()
    {
        let branch =
            String::from_utf8(output.stdout).expect("Couldn't parse the branch name from utf8");
        println!("cargo:rustc-env=GIT_BRANCH={}", branch.trim());
    }

    if let Ok(output) = Command::new("rustc").arg("--version").output()
        && output.status.success()
    {
        let rustc_version =
            String::from_utf8(output.stdout).expect("Couldn't parse the rust version from utf8");
        println!("cargo:rustc-env=RUSTC_VERSION={}", rustc_version.trim());
    }

    if let Ok(tc) = std::env::var("RUSTUP_TOOLCHAIN") {
        println!("cargo:rustc-env=RUST_TOOLCHAIN={}", tc);
    }

    if let Ok(output) = Command::new("cargo").arg("--version").output()
        && output.status.success()
    {
        let cargo_version =
            String::from_utf8(output.stdout).expect("Couldn't parse the cargo version from utf8");
        println!("cargo:rustc-env=CARGO_VERSION={}", cargo_version.trim());
    }
}
