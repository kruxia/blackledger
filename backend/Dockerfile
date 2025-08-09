# =======================================================================
# Build stage
# =======================================================================
FROM rust:1.89-slim AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /opt/blackledger

# Install sqlx-cli for migrations
RUN cargo install sqlx-cli --no-default-features --features postgres

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Build dependencies (this is cached as long as Cargo.toml doesn't change)
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm -rf src

# Copy source code
COPY src ./src
COPY migrations ./migrations
COPY .sqlx ./.sqlx

# Build the application
RUN touch src/main.rs
RUN cargo build --release

# =======================================================================
# deploy stage
# =======================================================================
FROM debian:bookworm-slim AS deploy

# Install deploy dependencies including PostgreSQL client for migrations
RUN apt-get update && apt-get install -y \
    libssl3 \
    postgresql-client \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /opt/blackledger

# Copy the binary from builder
COPY --from=builder /opt/blackledger/target/release/blackledger /usr/local/bin/blackledger

# Copy migrations
COPY --from=builder /opt/blackledger/migrations /opt/blackledger/migrations

# Install sqlx-cli for running migrations
COPY --from=builder /usr/local/cargo/bin/sqlx /usr/local/bin/sqlx

# Copy entrypoint script
COPY docker-entrypoint.sh /opt/blackledger/docker-entrypoint.sh
RUN chmod +x /opt/blackledger/docker-entrypoint.sh

# Create non-root user
RUN useradd -m -u 1000 blackledger && \
    chown -R blackledger:blackledger /opt/blackledger

USER blackledger

# Set environment variables
ENV RUST_LOG=blackledger=info,tower_http=info,sqlx=warn
ENV PORT=8000

EXPOSE 8000

# Use entrypoint to run migrations before starting
ENTRYPOINT ["/opt/blackledger/docker-entrypoint.sh"]
CMD ["blackledger"]

# =======================================================================
# Development stage
# =======================================================================
FROM rust:1.89 AS develop

WORKDIR /opt/blackledger

# Install cargo-watch for auto-reload, sqlx-cli for migrations, and cargo-llvm-cov for
# coverage testing
RUN cargo install cargo-watch \
    && cargo install sqlx-cli --no-default-features --features postgres \
    && rustup component add llvm-tools-preview \
    && cargo install cargo-llvm-cov

# Install PostgreSQL client for database connectivity checks
RUN apt-get update && apt-get install -y postgresql-client && rm -rf /var/lib/apt/lists/*

# Copy everything
COPY ./ ./

# Make entrypoint script executable
RUN chmod +x /opt/blackledger/docker-entrypoint.sh

# Set environment variables for development
ENV RUST_LOG=blackledger=debug,tower_http=debug
ENV DATABASE_URL=postgresql://postgres:password@postgres:5432/blackledger

EXPOSE 8000

# Use entrypoint script to run migrations before starting the app
ENTRYPOINT ["/opt/blackledger/docker-entrypoint.sh"]

# Use cargo-watch for auto-reload in development
CMD ["cargo", "watch", "-x", "run"]