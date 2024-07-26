#!/bin/bash
set -e
npm install
npm install --save-exact --save-dev esbuild
node_modules/.bin/esbuild --bundle src/index.ts --global-name=Pax --outfile=public/pax-interface-web.js
