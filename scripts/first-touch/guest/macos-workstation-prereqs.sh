#!/usr/bin/env bash
set -euo pipefail

ROOT="${PAX_FIRST_TOUCH_ROOT:-$HOME/Library/Logs/pax-first-touch}"
DONE_MARKER="$ROOT/workstation-prereqs.done"
FAILED_MARKER="$ROOT/workstation-prereqs.failed"
LOG_PATH="$ROOT/workstation-prereqs.log"
RUSTUP_TOOLCHAIN="${PAX_FIRST_TOUCH_RUSTUP_TOOLCHAIN:-stable}"
WASM_PACK_VERSION="${PAX_FIRST_TOUCH_WASM_PACK_VERSION:-0.15.0}"
NODE_VERSION="${PAX_FIRST_TOUCH_NODE_VERSION:-22.22.3}"
NODE_CACHE_DIR="${PAX_FIRST_TOUCH_NODE_CACHE_DIR:-$HOME/Library/Caches/pax-first-touch/node}"
NODE_DIST_BASE_URL="${PAX_FIRST_TOUCH_NODE_DIST_BASE_URL:-https://nodejs.org/dist}"
SUDO_PASSWORD="${PAX_FIRST_TOUCH_PASSWORD:-}"

mkdir -p "$ROOT"
export PATH="$HOME/.local/bin:$PATH"
if [[ -f "$HOME/.cargo/env" ]]; then
  # shellcheck source=/dev/null
  source "$HOME/.cargo/env"
fi

prereqs_ready() {
  xcode-select -p >/dev/null 2>&1 || return 1
  command -v rustup >/dev/null 2>&1 || return 1
  command -v rustc >/dev/null 2>&1 || return 1
  command -v cargo >/dev/null 2>&1 || return 1
  command -v node >/dev/null 2>&1 || return 1
  command -v npm >/dev/null 2>&1 || return 1
  command -v wasm-pack >/dev/null 2>&1 || return 1
  [[ "$(node --version)" == "v$NODE_VERSION" ]] || return 1
  [[ "$(wasm-pack --version)" == "wasm-pack $WASM_PACK_VERSION" ]] || return 1
  rustup target list --installed | grep -Fxq "wasm32-unknown-unknown" || return 1
}

if [[ -f "$DONE_MARKER" ]] && prereqs_ready; then
  exit 0
fi
rm -f "$FAILED_MARKER"
exec > >(tee -a "$LOG_PATH") 2>&1

sudo_cmd() {
  if [[ -n "$SUDO_PASSWORD" ]]; then
    printf '%s\n' "$SUDO_PASSWORD" | sudo -S "$@"
  else
    sudo "$@"
  fi
}

install_command_line_tools() {
  if xcode-select -p >/dev/null 2>&1; then
    return 0
  fi

  sudo_cmd touch /tmp/.com.apple.dt.CommandLineTools.installondemand.in-progress
  local label
  label="$(
    softwareupdate --list 2>&1 \
      | sed -n 's/^\* Label: \(.*Command Line Tools.*\)$/\1/p' \
      | tail -1
  )"
  if [[ -z "$label" ]]; then
    sudo_cmd rm -f /tmp/.com.apple.dt.CommandLineTools.installondemand.in-progress
    echo "Unable to find Command Line Tools in softwareupdate output." >&2
    exit 1
  fi

  sudo_cmd softwareupdate --install "$label" --verbose
  sudo_cmd rm -f /tmp/.com.apple.dt.CommandLineTools.installondemand.in-progress
}

install_node() {
  if command -v node >/dev/null 2>&1 \
    && command -v npm >/dev/null 2>&1 \
    && [[ "$(node --version)" == "v$NODE_VERSION" ]]; then
    return 0
  fi

  local machine platform node_name archive_name archive_path shasums expected_sha actual_sha
  machine="$(uname -m)"
  case "$machine" in
    arm64|aarch64)
      platform="darwin-arm64"
      ;;
    x86_64)
      platform="darwin-x64"
      ;;
    *)
      echo "Unsupported macOS architecture for Node.js: $machine" >&2
      exit 1
      ;;
  esac

  node_name="node-v$NODE_VERSION-$platform"
  archive_name="$node_name.tar.xz"
  archive_path="$NODE_CACHE_DIR/$archive_name"
  mkdir -p "$NODE_CACHE_DIR" "$HOME/.local/bin"

  if [[ ! -f "$archive_path" ]]; then
    curl --fail --location --show-error \
      --output "$archive_path" \
      "$NODE_DIST_BASE_URL/v$NODE_VERSION/$archive_name"
  fi

  shasums="$(curl --fail --location --show-error "$NODE_DIST_BASE_URL/v$NODE_VERSION/SHASUMS256.txt")"
  expected_sha="$(printf '%s\n' "$shasums" | awk -v archive="$archive_name" '$2 == archive { print $1; exit }')"
  if [[ -z "$expected_sha" ]]; then
    echo "Unable to find Node.js checksum for $archive_name." >&2
    exit 1
  fi
  actual_sha="$(shasum -a 256 "$archive_path" | awk '{ print $1 }')"
  if [[ "$actual_sha" != "$expected_sha" ]]; then
    rm -f "$archive_path"
    curl --fail --location --show-error \
      --output "$archive_path" \
      "$NODE_DIST_BASE_URL/v$NODE_VERSION/$archive_name"
    actual_sha="$(shasum -a 256 "$archive_path" | awk '{ print $1 }')"
    if [[ "$actual_sha" != "$expected_sha" ]]; then
      echo "Node.js archive SHA256 mismatch." >&2
      echo "Expected: $expected_sha" >&2
      echo "Actual:   $actual_sha" >&2
      exit 1
    fi
  fi

  rm -rf "$HOME/.local/$node_name"
  tar -xJf "$archive_path" -C "$HOME/.local"
  for bin in node npm npx corepack; do
    if [[ -e "$HOME/.local/$node_name/bin/$bin" ]]; then
      ln -sfn "$HOME/.local/$node_name/bin/$bin" "$HOME/.local/bin/$bin"
    fi
  done

  local path_line='export PATH="$HOME/.local/bin:$PATH"'
  for profile in "$HOME/.zprofile" "$HOME/.profile"; do
    touch "$profile"
    grep -Fxq "$path_line" "$profile" || printf '\n%s\n' "$path_line" >> "$profile"
  done
}

{
  sudo_cmd -v
  install_command_line_tools
  install_node

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

  {
    echo "pax-first-touch macOS system prerequisites ready"
    sw_vers -productVersion
    rustc --version
    cargo --version
    rustup --version
    node --version
    npm --version
    wasm-pack --version
    xcode-select -p
  } > "$DONE_MARKER"
} || {
  printf 'macOS prerequisite script failed. See %s\n' "$LOG_PATH" > "$FAILED_MARKER"
  exit 1
}
