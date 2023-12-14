#!/bin/bash

# Path to the pax-cli file
PAX_CLI="../pax/target/debug/pax-cli"

# Check if the pax-cli file exists
if [ ! -f "$PAX_CLI" ]; then
    echo "Warning: $PAX_CLI does not exist. You must first build pax-cli in the context of the ../pax directory (cargo build --workspace))"
    exit 1
else
    # Existing command from your script
    PAX_WORKSPACE_ROOT=../pax $PAX_CLI run --target=web --libdev --verbose
fi