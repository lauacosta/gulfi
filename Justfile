#just manual: https://github.com/casey/just/#readme

_default:
    @just --list

fmt: 
    cargo fmt

watch: 
    watchexec -r -e rs -- cargo run -- serve dev

udeps:
    rustup run nightly cargo udeps --all-targets --backend depinfo


# Ejecuta clippy
hack:
    cargo hack --feature-powerset --exclude-no-default-features clippy --locked -- -D warnings

# Ejecuta clippy
check:
    cargo clippy --locked -- -D warnings

# Ejecuta tests unitarios
test:
    cargo test --locked --all-features --all-targets

ci: fmt check hack test udeps
