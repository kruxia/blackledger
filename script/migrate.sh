#!/bin/bash

MIGRATION_KEY=${1:-$(sqly migrations blackledger | tail -1)}
$(dirname $0)/waitpg.sh
sqly migrate ${MIGRATION_KEY}
