#!/bin/bash

# Build
xcodebuild archive \
-configuration Debug \
-scheme pax-dev-harness-macos \
-archivePath ~/Library/Archives/PaxDevHarnessMacos.xcarchive \
-sdk macosx \
SKIP_INSTALL=NO BUILD_LIBRARIES_FOR_DISTRIBUTION=YES SUPPORTS_MACCATALYST=YES

# Run
#./debug/Products/Applications/pax-dev-harness-macos.app/Contents/MacOS/pax-dev-harness-macos
~/Library/Archives/PaxDevHarnessMacos.xcarchive/Products/Applications/pax-dev-harness-macos.app/Contents/MacOS/pax-dev-harness-macos

#see https://stackoverflow.com/questions/56978529/using-xcodebuild-to-do-a-command-line-builds-for-catalyst-uikit-for-mac/57402455#57402455

#xcodebuild  \
#-exportNotarizedApp \
#-sdk macosx \
#-configuration "Debug" \
#-exportPath . \
#-archivePath $HOME/Documents/Archives/macCatalyst.xcarchive \
#SKIP_INSTALL=NO BUILD_LIBRARIES_FOR_DISTRIBUTION=YES SUPPORTS_MACCATALYST=YES
#


#-workspace pax-dev-harness-macos.xcodeproj/project.xcworkspace \
#-scheme "pax-dev-harness-macos" \



#
#xcodebuild archive \
#-scheme $SCHEME \
#-archivePath $ARCHS/macCatalyst.xcarchive \
#-sdk macosx \
#SKIP_INSTALL=NO BUILD_LIBRARIES_FOR_DISTRIBUTION=YES SUPPORTS_MACCATALYST=YES


#Available destinations for the "pax-dev-harness-macos" scheme:
#                { platform:macOS, arch:arm64, id:00008103-001D24820A33001E }
#                { platform:macOS, arch:x86_64, id:00008103-001D24820A33001E }