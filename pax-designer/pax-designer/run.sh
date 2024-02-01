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

# Run cli & priv agent in parallell, and pipe both outputs to terminal
(PAX_WORKSPACE_ROOT=../pax PAX_CORP_ROOT=../ $PAX_CLI run --target=web --libdev --verbose) \
2>&1 | tee >(cd ../pax-privileged-agent && cargo run -- ../designer-project)
