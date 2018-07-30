#!/usr/bin/env bash

# https://www.gnu.org/software/bash/manual/html_node/The-Set-Builtin.html
# -e => Exit on error instead of continuing
# -v => Verbose - print input as it comes in. This means when you
#       run a script, the script itself will be printed as well. Useful
#       for understanding where Travis failed at.
set -ev

CURR_DIR=$(pwd)

CARGO_TARGET_DIR="$CURR_DIR/cache/target"
CARGO_HOME="$CURR_DIR/cache/cargo"

# Run all the tests with debug info + debug_asserts
cargo test
