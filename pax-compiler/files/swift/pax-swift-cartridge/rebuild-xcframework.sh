#!/bin/bash

# Extracts .frameworks from xcframework, rebuilds xcframework.
# Used to rely on xcodebuild's sanitary bundling process for the fragile xcframework structure

set -e

# An array of architectures
archs=("ios-arm64" "iossimulator-multiarch" "macos-multiarch")

# Iterate over each architecture and perform the operations
for arch in "${archs[@]}"; do
    mkdir -p $arch/PaxCartridge.framework
    cp -r PaxCartridge.xcframework/$arch/PaxCartridge.framework $arch/
done

rm -rf PaxCartridge.xcframework/

# Construct the xcodebuild command
cmd="xcodebuild -create-xcframework"

for arch in "${archs[@]}"; do
    cmd="$cmd -framework $arch/PaxCartridge.framework"
done

cmd="$cmd -output PaxCartridge.xcframework"

# Execute the xcodebuild command
eval $cmd

# Cleanup the temporary directories
for arch in "${archs[@]}"; do
    rm -rf $arch/
done