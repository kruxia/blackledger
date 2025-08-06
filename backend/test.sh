#!/bin/bash
set -e

echo "Setting up test database..."

# Start test database if not running
if ! docker compose ps | grep -q postgres-test; then
    docker compose up -d postgres-test
    echo "Waiting for test database to be ready..."
    sleep 5
fi

# Export test environment variables
export DATABASE_URL="postgresql://blackledger_test:test@localhost:5434/blackledger_test"
export TEST_DATABASE_URL="postgresql://blackledger_test:test@localhost:5434/blackledger_test"

# Create database if it doesn't exist
docker compose exec -T postgres-test psql -U blackledger_test -c "SELECT 1" 2>/dev/null || \
    docker compose exec -T postgres-test createdb -U blackledger_test blackledger_test

# Run migrations
echo "Running migrations..."
sqlx migrate run

# Run tests
echo "Running tests..."
cargo test --all-features -- --test-threads=1

echo "Tests completed successfully!"