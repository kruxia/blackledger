#!/bin/bash
DEFAULT_PATHS="blackledger script tests"

isort --profile black ${@:-$DEFAULT_PATHS}
black ${@:-$DEFAULT_PATHS}