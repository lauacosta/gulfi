_default:
    just --list

run:
    cargo run --release -- serve

udeps:
    cargo udeps --all-targets --backend depinfo

all: run

sync:
    cargo run --quiet --release -- sync 

embed input:
    cargo run --release -- embed --model open-ai --input {{ input }}
    
clippy:
    cargo clippy -- -Aclippy::pedantic
