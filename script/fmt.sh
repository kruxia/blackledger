#!/bin/bash
DEFAULT_PATHS="blackledger script"

ruff check --select I --fix ${@:-$DEFAULT_PATHS}
ruff format ${@:-$DEFAULT_PATHS}