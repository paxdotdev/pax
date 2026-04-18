#!/bin/bash
set -euo pipefail

ESBUILD_BIN="./node_modules/.bin/esbuild"
if [ ! -x "$ESBUILD_BIN" ] || [ ! -d "./node_modules/snarkdown" ] || [ ! -d "./node_modules/html2canvas" ]; then
  npm install --no-audit --no-fund --no-package-lock
fi

if [ ! -x "$ESBUILD_BIN" ]; then
  ESBUILD_BIN="$(command -v esbuild || true)"
fi
if [ -z "${ESBUILD_BIN}" ]; then
  echo "Unable to find esbuild. Run npm install in $(pwd) and try again." >&2
  exit 1
fi

"$ESBUILD_BIN" --bundle src/index.ts --global-name=Pax --outfile=public/pax-interface-web.js
