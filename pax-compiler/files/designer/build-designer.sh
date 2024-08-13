#!/bin/bash

set -e

# Defaults expecting to be called from same directory within monorepo
PAX_DESIGNER_PATH=${1:-'../../../pax-designer'}
OUTPUT_FILE_PATH=${2:-'./manifest.json'}

# Running cargo and redirecting output to OUTPUT_FILE_PATH
cargo run --manifest-path="$PAX_DESIGNER_PATH/Cargo.toml" --features=parser --bin=parser > "$OUTPUT_FILE_PATH"
