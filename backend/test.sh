#!/bin/bash
set -e

echo "Setting up test database..."

# Recreate test database
docker compose down -v postgres-test || true
docker compose up -d postgres-test

# Export test environment variables
export DATABASE_URL="postgresql://blackledger_test:test@localhost:5434/blackledger_test"
export TEST_DATABASE_URL="postgresql://blackledger_test:test@localhost:5434/blackledger_test"

# Wait for test database to become available
until psql -c "SELECT true;" $TEST_DATABASE_URL &>/dev/null; do
    echo "Waiting for test database to be available..."
    sleep 1
done

# Run migrations
echo "Running migrations..."
sqlx migrate run

# Run tests
echo "Running tests..."
# Run tests sequentially to avoid database state conflicts
RUST_BACKTRACE=1 cargo test --all-features -- --test-threads=1
# cargo tarpaulin --config tarpaulin.toml --out Html --all-features -- --test-threads=1

echo "Tests completed successfully!"