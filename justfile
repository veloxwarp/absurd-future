# List all recipes
default:
	just --list --unsorted

# Run example
example-tokio:
	cargo run --example tokio

# cargo compile
cargo-compile:
    cargo test --workspace --no-run --locked

# Clippy check
cargo-clippy-check:
    cargo clippy --no-deps --workspace --locked --tests --benches --examples -- -Dwarnings

# Rustfmt check
cargo-fmt-check:
    cargo fmt --all --check

# Test
test:
	cargo test --workspace --locked
