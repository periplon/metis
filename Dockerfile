# Rust build stage
FROM rust:1.75 as builder

WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src ./src

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
