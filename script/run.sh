#!/bin/bash
PACKAGE_DIR=$(dirname $(dirname $0))
cd $PACKAGE_DIR
cargo-watch -w src --poll -x run