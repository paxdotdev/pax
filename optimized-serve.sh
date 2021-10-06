#!/bin/sh

set -ex

# Run the `wasm-pack` CLI tool to build and process the Rust wasm file

# Manual command is commented below; note difference in relative manifest path:
# cargo build --manifest-path=piet/Cargo.toml --all-targets --all-features



# See this article for ideas on space saving:
# https://rustwasm.github.io/book/reference/code-size.html#optimizing-builds-for-code-size

cd carbon-chassis-web
wasm-pack build --release -d basic-web-static/dist
# wasm-opt was installed with `brew install binaryen`
wasm-opt -Oz -o basic-web-static/dist/carbon_chassis_web_bg_opt.wasm basic-web-static/dist/carbon_chassis_web_bg.wasm
#mv basic-web-static/dist/carbon_chassis_web_bg_opt.wasm basic-web-static/dist/carbon_chassis_web_bg.wasm
##TODO:  gzip

# Finally, package everything up using Webpack and start a server so we can
# browse the result
cd basic-web-static
yarn serve || (yarn && yarn serve)
