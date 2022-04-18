#!/bin/bash

# Build
xcodebuild archive \
-configuration Debug \
-scheme pax-dev-harness-macos \
-archivePath ~/Library/Archives/PaxDevHarnessMacos.xcarchive \
-sdk macosx \
SKIP_INSTALL=NO BUILD_LIBRARIES_FOR_DISTRIBUTION=YES SUPPORTS_MACCATALYST=YES

# Run
~/Library/Archives/PaxDevHarnessMacos.xcarchive/Products/Applications/pax-dev-harness-macos.app/Contents/MacOS/pax-dev-harness-macos
