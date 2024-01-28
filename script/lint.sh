#!/bin/bash
DEFAULT_PATHS="blackledger script"

isort --profile black --check ${@:-$DEFAULT_PATHS}
black --check ${@:-$DEFAULT_PATHS}
flake8 ${@:-$DEFAULT_PATHS}
