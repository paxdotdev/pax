#!/bin/bash
set -euo pipefail

# Reconcile existing installs too: a cached executable may predate security updates.
npm ci --include=dev --no-audit --no-fund

node ./build-interface.mjs
