#!/bin/bash
set -euo pipefail

# Reconcile existing installs too: a cached executable may predate security updates.
npm ci --include=dev --no-audit --no-fund

./node_modules/.bin/esbuild --bundle src/index.ts --global-name=Pax --outfile=public/pax-interface-web.js
