default:
    @just --list

ci:
    cargo fmt --all -- --check
    cargo clippy --locked --all-targets -- -D warnings
    cargo nextest run --locked
