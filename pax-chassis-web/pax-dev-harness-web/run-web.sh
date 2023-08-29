#!/bin/sh
SHOULD_ALSO_RUN=$1
OUTPUT_PATH=$2

assets_dir="../../../../assets"
new_dir="./public/assets"
mkdir -p "$new_dir"
cp -r "$assets_dir"/* "$new_dir"

#npm run build

# Clear old build and move to output directory
rm -rf "$OUTPUT_PATH"
mkdir -p "$OUTPUT_PATH"
cp -r dist "$OUTPUT_PATH"

if [ "$SHOULD_ALSO_RUN" = "true" ]; then
  # Run
  set -ex
  yarn serve || (yarn && yarn serve)
fi
