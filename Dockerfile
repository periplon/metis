# UI build stage - use bookworm variant to match runtime glibc
FROM rust:1.91-bookworm AS frontend-builder

WORKDIR /app

# Install wasm target and cargo-leptos
RUN rustup target add wasm32-unknown-unknown
RUN cargo install cargo-leptos --version 0.2.47

# Copy workspace files and all source code
COPY Cargo.toml Cargo.lock ./
COPY ui ./ui
COPY src ./src
COPY benches ./benches

# Build UI from workspace root
RUN cargo leptos build --release

# Rust backend build stage - use bookworm variant to match runtime glibc
FROM rust:1.91-bookworm AS builder

WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src ./src

# Copy UI source for workspace resolution
COPY ui ./ui

# Copy benches for Cargo.toml validation
COPY benches ./benches

# Copy built UI assets
# cargo-leptos outputs to target/site in the workspace target directory usually
# Since we ran it in /app/ui, it might be in /app/ui/target/site or /app/target/site depending on configuration
# Let's assume default behavior relative to the cargo.toml used.
COPY --from=frontend-builder /app/target/site ./ui/dist

# Build release binary
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y ca-certificates curl && \
    rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/metis /usr/local/bin/metis

# Copy example configurations
COPY examples /app/examples

# Create config directory
RUN mkdir -p /app/config

# Expose port
EXPOSE 3000

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:3000/health/live || exit 1

# Run as non-root user
RUN useradd -m -u 1000 metis && \
    chown -R metis:metis /app
USER metis

# Default command
CMD ["metis", "--config", "/app/config/metis.toml"]
