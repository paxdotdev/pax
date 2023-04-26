#!/bin/bash

ASSET_TYPE="$1"
ASSET_FILE_NAME="$2"
ASSET_FILE_SOURCE_PATH="$3"

if [ -z "$ASSET_TYPE" ] || [ -z "$ASSET_FILE_NAME" ] || [ -z "$ASSET_FILE_SOURCE_PATH" ]; then
    echo "Usage: ./add_asset.sh <asset_type> <asset_file_name> <asset_file_source_path>"
    exit 1
fi

PROJECT_DIR="YourProjectName"

# Create the target directory based on asset type
if [ "$ASSET_TYPE" == "font" ]; then
    TARGET_DIR="$PROJECT_DIR/Fonts"
elif [ "$ASSET_TYPE" == "image" ]; then
    TARGET_DIR="$PROJECT_DIR/Images"
else
    echo "Invalid asset type. Supported types are: font, image"
    exit 1
fi

mkdir -p "$TARGET_DIR"

# Copy the asset file to the target directory
cp "$ASSET_FILE_SOURCE_PATH" "$TARGET_DIR"

if [ "$ASSET_TYPE" == "font" ]; then
    # Update the Info.plist to include the font
    INFO_PLIST="$PROJECT_DIR/Info.plist"
    FONT_KEY="<key>UIAppFonts</key>"

    if ! grep -q "$FONT_KEY" "$INFO_PLIST"; then
        # Insert the font key and array structure
        sed -i.bak "/<\/dict>/i\\
        $FONT_KEY\\
        <array>\\
        </array>" "$INFO_PLIST"
    fi

    FONT_ITEM="<string>$ASSET_FILE_NAME</string>"

    if ! grep -q "$FONT_ITEM" "$INFO_PLIST"; then
        # Insert the font file name into the array
        sed -i.bak "/<array>/a\\
        $FONT_ITEM" "$INFO_PLIST"
    fi

    # Remove backup file
    rm "$INFO_PLIST.bak"
fi

echo "Asset added to the project successfully."
