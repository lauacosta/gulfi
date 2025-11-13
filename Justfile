#just manual: https://github.com/casey/just/#readme

_default:
    @just --list

build:build-ui
    cargo build --bin gulfi

# Runs cargo fmt
fmt: 
    cargo fmt

watch: 
    watchexec -r -e rs -- cargo run -- serve dev

# Searches for unused dependencies
udeps:
    RUSTC_BOOTSTRAP=1 cargo udeps --all-targets --backend depinfo

# Runs cargo hack
hack:
    cargo hack check --feature-powerset --no-dev-deps --exclude-no-default-features 

# Runs clippy
check:
    cargo clippy --locked -- -D warnings -D clippy::unwrap_used 

# Runs the test suite
test:
    cargo nextest r --locked --all-features --all-targets --profile ci

# Builds the UI
build-ui:
    cargo run --bin xtask -- build-frontend

# Runs cargo-deny
deny:
    cargo-deny --all-features check

# Runs cargo-audit
audit:
    cargo audit

ci: fmt check test udeps audit deny build-ui

validate-ci:
    circleci config validate
