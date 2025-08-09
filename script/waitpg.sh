#!/bin/bash
set -eux

# wait for postgres to be up
until psql $DATABASE_URL -c '\dt' --pset=pager=off; do
    >&2 echo "Waiting for postgres, give us a second..."
    sleep 1
done
