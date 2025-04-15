_default:
    just --list

dev:
    cargo run -- serve dev

watch: 
    watchexec -r -e rs -- cargo run -- serve dev

build: 
    watchexec -r -e rs -- cargo build

run *args:
    cargo run -- {{args}}

udeps:
    cargo udeps --all-targets --backend depinfo

all: run

clippy:
    cargo clippy -- -Aclippy::pedantic
