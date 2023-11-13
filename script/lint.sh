#!/bin/bash

isort --profile black --check ${@:-.}
black --check ${@:-.}
flake8 ${@:-.}
