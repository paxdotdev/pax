#!/bin/bash
source helpers.sh

# Path to the pax-cli file
PAX_CLI="../pax/target/debug/pax-cli"

add_designtime_dependency

# Build pax-cli with designtime flag
pushd ../pax/pax-cli
cargo build --features="designtime"
popd

remove_designtime_dependency

# Run cli
PAX_WORKSPACE_ROOT=../pax PAX_CORP_ROOT=../ $PAX_CLI run --target=web --libdev --verbose
