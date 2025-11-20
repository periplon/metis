# justfile
#
# just is a handy command runner.
# See https://github.com/casey/just

# Build the project
build:
	cargo build

# Run tests
test:
	cargo test

# Run the project
run:
	cargo run

# Format code
fmt:
	cargo fmt --all

# Lint code
clippy:
	cargo clippy -- -D warnings

# Clean the project
clean:
	cargo clean

# Run the UI development server
ui-dev:
	cd ui && cargo leptos watch

# Build the UI for production
ui-build:
	cd ui && cargo leptos build --release

# Run the full application with UI
start-full:
	just build
	just ui-build
	RUST_LOG=info cargo run --all-features
