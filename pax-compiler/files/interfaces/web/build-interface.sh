#!/bin/bash
set -euo pipefail

ESBUILD_BIN="./node_modules/.bin/esbuild"
if [ ! -x "$ESBUILD_BIN" ]; then
  ESBUILD_BIN="$(command -v esbuild || true)"
fi

if [ -z "${ESBUILD_BIN}" ]; then
  npm install --only=production
  ESBUILD_BIN="./node_modules/.bin/esbuild"
fi

"$ESBUILD_BIN" --bundle src/index.ts --global-name=Pax --outfile=public/pax-interface-web.js
