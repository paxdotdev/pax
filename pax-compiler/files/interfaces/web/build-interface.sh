#!/bin/bash
set -e
npm install -g esbuild
npm install --only=production
esbuild --bundle src/index.ts --global-name=Pax --outfile=public/pax-interface-web.js