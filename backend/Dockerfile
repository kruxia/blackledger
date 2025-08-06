# Build stage
FROM rust:1.88-slim AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Build dependencies (this is cached as long as Cargo.toml doesn't change)
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm -rf src

# Copy source code
COPY src ./src
COPY migrations ./migrations

# Build the application
RUN touch src/main.rs
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim AS runtime

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    libpq5 \
    libssl3 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the binary from builder
COPY --from=builder /app/target/release/blackledger /usr/local/bin/blackledger

# Copy migrations
COPY --from=builder /app/migrations /app/migrations

# Create non-root user
RUN useradd -m -u 1000 blackledger && \
    chown -R blackledger:blackledger /app

USER blackledger

# Set environment variables
ENV RUST_LOG=blackledger=info,tower_http=info,sqlx=warn
ENV PORT=8000

EXPOSE 8000

CMD ["blackledger"]

# Development stage
FROM rust:1.88 AS develop

WORKDIR /app

# Install cargo-watch for auto-reload
RUN cargo install cargo-watch sqlx-cli

# Install PostgreSQL client
RUN apt-get update && apt-get install -y postgresql-client && rm -rf /var/lib/apt/lists/*

# Copy everything
COPY . .

# Set environment variables for development
ENV RUST_LOG=blackledger=debug,tower_http=debug
ENV DATABASE_URL=postgresql://postgres:password@postgres:5432/blackledger

EXPOSE 8000

# Use cargo-watch for auto-reload in development
CMD ["cargo", "watch", "-x", "run"]