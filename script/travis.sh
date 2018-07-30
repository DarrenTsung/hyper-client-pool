#!/usr/bin/env bash

# https://www.gnu.org/software/bash/manual/html_node/The-Set-Builtin.html
# -e => Exit on error instead of continuing
set -e

CURR_DIR=$(pwd)

CARGO_TARGET_DIR="$CURR_DIR/cache/target"
CARGO_HOME="$CURR_DIR/cache/cargo"

# Run all the tests with debug info + debug_asserts
cargo test
