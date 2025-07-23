#just manual: https://github.com/casey/just/#readme

_default:
    @just --list

# Runs cargo fmt
fmt: 
    cargo fmt

watch: 
    watchexec -r -e rs -- cargo run -- serve dev


# Searches for unused dependencies
udeps:
    RUSTC_BOOTSTRAP=1 cargo udeps --all-targets --backend depinfo

# Searches for unused dependencies and generates a json report
udeps_report:
    RUSTC_BOOTSTRAP=1 cargo udeps --all-targets --backend depinfo --output json > udeps.json

# Runs cargo hack
hack:
    cargo hack check --feature-powerset --no-dev-deps --exclude-no-default-features 

# Runs clippy
check:
    cargo clippy --locked -- -D warnings -D clippy::unwrap_used 

# Runs clippy and generates a json report
check_report:
    cargo clippy --locked --message-format=json -- -D warnings -D clippy::unwrap_used  > clippy_raw.json
    cargo deduplicate-warnings < clippy_raw.json > clippy.json

# Runs the test suite
test update="":
  {{ if update == "update" { "UPDATE_EXPECT=1" } else { "" } }}  cargo nextest r --locked --all-features --all-targets --profile ci
  

# Builds the UI
build-ui:
    cd ./crates/gulfi-server/ui/ && pnpm build

# Runs cargo-deny
deny:
    cargo-deny --all-features check

# Runs cargo-deny and generates a json report 
deny_report:
    cargo deny --all-features --format json check 2> deny.json

# Runs cargo-audit
audit:
    cargo audit

# Runs cargo-audit and generates a json report 
audit_report:
    cargo audit --json > audit.json

sonar:
    cargo sonar --audit --clippy --deny --udeps

coverage:
    cargo llvm-cov --locked --lcov --output-path lcov.info

ci: fmt check test udeps audit deny build-ui

validate-ci:
    circleci config validate
