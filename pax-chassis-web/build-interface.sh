#!/bin/bash
set -e
pushd interface
esbuild --bundle src/index.ts --global-name=Pax --outfile=public/pax-chassis-web-interface.js
popd
