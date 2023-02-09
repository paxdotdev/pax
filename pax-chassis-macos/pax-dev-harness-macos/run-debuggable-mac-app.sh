#!/bin/bash

# Build
xcodebuild archive \
-configuration Debug \
-scheme "Pax macOS" \
-archivePath ~/Library/Archives/PaxDevHarnessMacos.xcarchive \
-sdk macosx \
SKIP_INSTALL=NO BUILD_LIBRARIES_FOR_DISTRIBUTION=YES SUPPORTS_MACCATALYST=YES

# Run
~/Library/Archives/PaxDevHarnessMacos.xcarchive/Products/Applications/Pax\ macOS.app/Contents/MacOS/Pax\ macOS
