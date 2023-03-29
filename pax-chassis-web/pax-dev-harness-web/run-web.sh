#!/bin/sh

SHOULD_ALSO_RUN=$1
OUTPUT_PATH=$2

# Clear old build and move to output directory
rm -rf $OUTPUT_PATH
mkdir -p $OUTPUT_PATH
cp -r "." $OUTPUT_PATH
cd $OUTPUT_PATH

# Remove this script in output directory
rm -- "$0"

if [ "$SHOULD_ALSO_RUN" == "true" ]; then
  # Run
  set -ex
  yarn serve || (yarn && yarn serve)
fi

