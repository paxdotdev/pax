#!/usr/bin/env bash
set -euo pipefail

RUSTUP_TOOLCHAIN="${PAX_FIRST_TOUCH_RUSTUP_TOOLCHAIN:-stable}"
WASM_PACK_VERSION="${PAX_FIRST_TOUCH_WASM_PACK_VERSION:-0.15.0}"

if [[ ! -x "$HOME/.cargo/bin/rustup" ]]; then
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
    | sh -s -- -y --profile default --default-toolchain "$RUSTUP_TOOLCHAIN"
fi

# shellcheck source=/dev/null
source "$HOME/.cargo/env"

rustup toolchain install "$RUSTUP_TOOLCHAIN"
rustup default "$RUSTUP_TOOLCHAIN"
rustup target add wasm32-unknown-unknown

if ! command -v wasm-pack >/dev/null 2>&1 \
  || [[ "$(wasm-pack --version)" != "wasm-pack $WASM_PACK_VERSION" ]]; then
  cargo install wasm-pack --version "$WASM_PACK_VERSION"
fi

rustc --version
cargo --version
node --version
npm --version
wasm-pack --version
