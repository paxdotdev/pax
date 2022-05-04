#!/bin/bash

cargo build
cd pax-chassis-macos/pax-dev-harness-macos/ || return
./run-debuggable-mac-app.sh