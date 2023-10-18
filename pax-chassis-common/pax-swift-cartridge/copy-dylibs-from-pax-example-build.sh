#!/bin/bash

# Extracts .frameworks from xcframework, rebuilds xcframework.
# Used to rely on xcodebuild's sanitary bundling process for the fragile xcframework structure

set -e

cp -r ../../pax-example/.pax/pkg/pax-chassis-common/pax-swift-cartridge/PaxCartridge.xcframework/ ./PaxCartridge.xcframework/