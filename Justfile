#just manual: https://github.com/casey/just/#readme

_default:
    @just --list

# Formateo el texto.
fmt: 
    cargo fmt

watch: 
    watchexec -r -e rs -- cargo run -- serve dev

# Busco dependencias que no estan siendo usadas.
udeps:
    RUSTC_BOOTSTRAP=1 cargo udeps --all-targets --backend depinfo

hack:
    cargo hack --feature-powerset --exclude-no-default-features clippy --locked -- -D warnings

# Ejecuta clippy
check:
    cargo clippy --locked -- -D warnings -D clippy::unwrap_used

# Ejecuta la suite de testeos.
test:
    cargo test --locked --all-features --all-targets

build-ui:
    cd ./crates/gulfi-server/ui/ && pnpm build

deny:
    cargo-deny --all-features check

ci: fmt check hack test udeps deny build-ui

validate-ci:
    circleci config validate
