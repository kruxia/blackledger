#!/bin/bash

MIGRATION_KEY=${1:-$(sqly migrations blackledger | tail -1)}
sqly migrate ${MIGRATION_KEY}
