#!/bin/bash
DEFAULT_PATHS="blackledger script"

ruff check ${@:-$DEFAULT_PATHS}
