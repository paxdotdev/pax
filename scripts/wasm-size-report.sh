#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 1 || $# -gt 2 ]]; then
  echo "usage: $0 <path-to.wasm> [top-count]" >&2
  exit 64
fi

export LC_ALL=C
export TZ=UTC

wasm_path="$1"
top_count="${2:-40}"
twiggy_bin="${TWIGGY:-twiggy}"

if [[ ! -f "$wasm_path" ]]; then
  echo "wasm file not found: $wasm_path" >&2
  exit 66
fi

version_output="$("$twiggy_bin" --version 2>/dev/null || true)"
version="$(printf '%s\n' "$version_output" | awk '{print $2}')"
major="$(printf '%s\n' "$version" | cut -d. -f1)"
minor="$(printf '%s\n' "$version" | cut -d. -f2)"

if [[ -z "$version" || -z "$major" || -z "$minor" ]]; then
  echo "failed to determine twiggy version from: $version_output" >&2
  exit 69
fi

if (( major == 0 && minor < 8 )); then
  echo "twiggy 0.8.0 or newer is required; found $version_output" >&2
  echo "install with: cargo install twiggy --version 0.8.0 --locked" >&2
  exit 69
fi

raw_bytes="$(wc -c < "$wasm_path" | tr -d ' ')"
gzip_bytes="$(gzip -cn -9 "$wasm_path" | wc -c | tr -d ' ')"

echo "wasm: $wasm_path"
echo "twiggy: $version_output"
echo "raw-bytes: $raw_bytes"
echo "gzip-9-bytes: $gzip_bytes"
echo "named-release-tip: build with PAX_RELEASE_KEEP_SYMBOLS=1 for named Twiggy release reports"

if command -v brotli >/dev/null 2>&1; then
  brotli_bytes="$(brotli -q 11 -c "$wasm_path" | wc -c | tr -d ' ')"
  echo "brotli-11-bytes: $brotli_bytes"
fi

echo
echo "== twiggy top =="
"$twiggy_bin" top -n "$top_count" "$wasm_path"

echo
echo "== twiggy retained =="
"$twiggy_bin" top --retained -n "$top_count" "$wasm_path"

echo
echo "== twiggy monos =="
"$twiggy_bin" monos -n "$top_count" "$wasm_path"
