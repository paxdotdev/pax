#!/bin/bash

# Path to the pax-cli file
PAX_CLI="../pax/target/debug/pax-cli"

# Build pax-cli with designtime flag
pushd ../pax/pax-cli
cargo build --features="designtime"
popd

# Existing command from your script
PAX_WORKSPACE_ROOT=../pax $PAX_CLI run --target=web --libdev --verbose
