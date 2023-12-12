#!/bin/bash
DEFAULT_PATHS="blackledger script tests"

isort --profile black --check ${@:-$DEFAULT_PATHS}
black --check ${@:-$DEFAULT_PATHS}
flake8 ${@:-$DEFAULT_PATHS}
