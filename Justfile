_default:
    just --list

dev:
    cargo run -- serve dev

watch: 
    watchexec -r -e rs -- cargo run

run *args:
    cargo run --release -- {{args}}

udeps:
    cargo udeps --all-targets --backend depinfo

all: run

clippy:
    cargo clippy -- -Aclippy::pedantic
