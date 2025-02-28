_default:
    just --list

run *args:
    cargo run --release -- {{args}}

udeps:
    cargo udeps --all-targets --backend depinfo

all: run

clippy:
    cargo clippy -- -Aclippy::pedantic
