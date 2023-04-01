#!/bin/bash

# Expects args:
# 1: VERBOSE ∈ {"true" | "false"}
# 2: EXCLUDE_ARCHS ∈ {"arm64" | "x86_64"}
# 3: SHOULD_ALSO_RUN ∈ {"true" | "false"}
# 4: OUTPUT_PATH : output directory for build
VERBOSE=$1
EXCLUDE_ARCHS=$2
SHOULD_ALSO_RUN=$3
OUTPUT_PATH=$4


runbuild () {
  xcodebuild archive \
  -configuration Debug \
  -scheme "Pax macOS" \
  -archivePath build/PaxDevHarnessMacos.xcarchive \
  -sdk macosx13.3 \
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

# Clear old build and move to output directory
rm -rf $OUTPUT_PATH
mkdir -p $OUTPUT_PATH
cp -r "build/PaxDevHarnessMacos.xcarchive/Products/Applications/Pax macOS.app" $OUTPUT_PATH
cd $OUTPUT_PATH

if [ "$SHOULD_ALSO_RUN" == "true" ]; then
  # Run
  Pax\ macOS.app/Contents/MacOS/Pax\ macOS
fi