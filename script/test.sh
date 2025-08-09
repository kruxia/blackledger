#!/bin/bash
set -eu
DIR=$(dirname $0)

# wait for postgres to be up
$DIR/waitpg.sh

# drop/create the test database
psql -c "DROP DATABASE ${DATABASE_NAME}_test" $DATABASE_URL || true
psql -c "CREATE DATABASE ${DATABASE_NAME}_test" $DATABASE_URL || true

# migrate the database
export DATABASE_URL="${DATABASE_URL}_test"
$DIR/migrate.sh

# run pytests
pytest $@
