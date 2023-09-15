#!/bin/sh
SHOULD_ALSO_RUN=$1
OUTPUT_PATH=$2

assets_dir="../../../../assets"
new_dir="./public/assets"
mkdir -p "$new_dir"


if [ -d "$assets_dir" ]; then
  cp -r "$assets_dir"/* "$new_dir"
fi


if [ "$SHOULD_ALSO_RUN" = "true" ]; then
  # Run, which doesn't require a previous build step due to webpack dev server
  set -ex
  yarn serve || (yarn && yarn serve)
else
  # Build, with production webpack config
  yarn && yarn build

  # Clear old build and move to output directory
  rm -rf "$OUTPUT_PATH"
  mkdir -p "$OUTPUT_PATH"
  cp -r dist "$OUTPUT_PATH"
fi


