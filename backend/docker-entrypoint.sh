#!/bin/bash
set -e

echo "Waiting for database to be ready..."
until pg_isready -h "${DATABASE_HOST:-postgres-backend}" -p "${DATABASE_PORT:-5432}" -U "${DATABASE_USER:-postgres}"; do
  echo "Database is unavailable - sleeping"
  sleep 1
done

echo "Database is ready!"

# Run migrations
echo "Running database migrations..."
sqlx migrate run

echo "Migrations complete!"

# Execute the main command
exec "$@"