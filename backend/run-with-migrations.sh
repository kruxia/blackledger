#!/bin/bash
# Script to run migrations and start the Rust backend locally
set -e

echo "Running database migrations..."
sqlx migrate run

echo "Migrations complete! Starting server..."
cargo run