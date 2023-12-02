#!/bin/bash
DEFAULT_PATHS="blackledger tests"

isort --profile black ${@:-$DEFAULT_PATHS}
black ${@:-$DEFAULT_PATHS}