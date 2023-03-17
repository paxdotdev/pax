#!/bin/bash

# Expects args:
# 1: VERBOSE ∈ {true | false}
# 2: EXCLUDE_ARCHS ∈ {"arm64" | "x86_64"}
VERBOSE=$1
EXCLUDE_ARCHS=$2

runbuild () {
  xcodebuild archive \
  -configuration Debug \
  -scheme "Pax macOS" \
  -archivePath build/PaxDevHarnessMacos.xcarchive \
  -sdk macosx13.1 \
  SKIP_INSTALL=NO SUPPORTS_MACCATALYST=YES ONLY_ACTIVE_ARCH=YES EXCLUDED_ARCHS="$EXCLUDE_ARCHS"
}

# Build
if [ "$VERBOSE" == "true" ]; then
  # Pipe output to both stdout and stderr
  runbuild 2>&1
else
  # Pipe stdout to /dev/null, but pipe stderr to /dev/stderr
  runbuild > /dev/null 2>&1 >&2
fi

# Run
build/PaxDevHarnessMacos.xcarchive/Products/Applications/Pax\ macOS.app/Contents/MacOS/Pax\ macOS
