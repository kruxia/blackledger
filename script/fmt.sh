#!/bin/bash
DEFAULT_PATHS="blackledger script"

isort --profile black ${@:-$DEFAULT_PATHS}
black ${@:-$DEFAULT_PATHS}