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

# Run cli & priv agent in parallel, and pipe both outputs to terminal
(cd ../pax-privileged-agent && cargo run -- ../designer-project) \
2>&1 | tee >(PAX_WORKSPACE_ROOT=../pax PAX_CORP_ROOT=../ $PAX_CLI run --target=web --libdev --verbose)

#terminate the priv agent if it for some reason is still alive
lsof -ti:8252 | xargs -r kill -15
