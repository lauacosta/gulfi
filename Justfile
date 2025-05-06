_default:
    just --list

run *args:
    cargo run -- {{args}}

watch: 
    watchexec -r -e rs -- cargo run -- serve dev

build-watch: 
    watchexec -r -e rs -- cargo build

udeps:
    cargo udeps --all-targets --backend depinfo

pedantic:
    cargo clippy -- -Aclippy::pedantic

ci: fmt clippy test-doc test

clippy:
    cargo clippy 

fmt: 
    cargo fmt

test:
    cargo test --locked --all-features --all-targets

test-doc:
    cargo test --locked --all-features --doc

# semver:
#     cargo semver-checks

# doc:
#     cargo docs-rs
