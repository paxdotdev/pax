#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PAX_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
VM_NAME="${PAX_FIRST_TOUCH_VM_NAME:-pax-macos-first-touch}"
VM_USER="${PAX_FIRST_TOUCH_USERNAME:-pax}"
VM_PASSWORD="${PAX_FIRST_TOUCH_PASSWORD:-}"
TMP_DIR="${PAX_FIRST_TOUCH_TMP_DIR:-/tmp/pax-first-touch/macos-source-smoke}"
GUEST_SRC="${PAX_FIRST_TOUCH_GUEST_SRC:-/Users/$VM_USER/pax-src}"
SMOKE_DIR="${PAX_FIRST_TOUCH_SMOKE_DIR:-/Users/$VM_USER/pax-smoke-source}"
DEFAULT_RUN_PORT="${PAX_FIRST_TOUCH_RUN_PORT:-8080}"
GUEST_TMP_DIR="${PAX_FIRST_TOUCH_GUEST_TMP_DIR:-/tmp/pax-first-touch-macos-source-smoke}"

if [[ -z "$VM_PASSWORD" ]]; then
  echo "Set PAX_FIRST_TOUCH_PASSWORD to the macOS VM password." >&2
  exit 1
fi

rm -rf "$TMP_DIR"
mkdir -p "$TMP_DIR"

COPYFILE_DISABLE=1 tar \
  --exclude '.git' \
  --exclude 'target' \
  --exclude '*/target' \
  --exclude '.build' \
  --exclude '*/.build' \
  --exclude 'node_modules' \
  --exclude '.DS_Store' \
  --exclude '._*' \
  --exclude '*/._*' \
  --exclude '__MACOSX' \
  --exclude '*/__MACOSX' \
  -czf "$TMP_DIR/pax-src.tar.gz" \
  -C "$PAX_ROOT" .

cat > "$TMP_DIR/smoke-macos-source.sh" <<'GUEST_SCRIPT'
#!/usr/bin/env bash
set -euo pipefail

ARCHIVE_PATH="$1"
SOURCE_PATH="$2"
SMOKE_PATH="$3"
DEFAULT_RUN_PORT="$4"

source "$HOME/.cargo/env"
export PATH="$HOME/.local/pax-source/bin:$HOME/.local/bin:$PATH"

rm -rf "$SOURCE_PATH" "$SMOKE_PATH"
mkdir -p "$SOURCE_PATH"
tar -xzf "$ARCHIVE_PATH" -C "$SOURCE_PATH"

mkdir -p "$HOME/.local/pax-source" "$HOME/.cache/pax-target"
CARGO_TARGET_DIR="$HOME/.cache/pax-target/pax-cli" \
  cargo install --path "$SOURCE_PATH/pax-cli" --root "$HOME/.local/pax-source" --force

pax-cli create "$SMOKE_PATH"
mkdir -p "$SMOKE_PATH/.cargo"
cat > "$SMOKE_PATH/.cargo/config.toml" <<EOF
[patch.crates-io]
pax-kit = { path = "$SOURCE_PATH/pax-kit" }
EOF

cd "$SMOKE_PATH"
pax-cli build --target web

if grep -RIn "occlusionLayer\\|occlusion_layer" .pax/interface/web .pax/build/debug/web 2>/dev/null; then
  echo "Legacy occlusion layer symbols leaked into the generated web interface." >&2
  exit 1
fi

run_log="$(mktemp)"
pax-cli run --target web >"$run_log" 2>&1 &
run_pid=$!
trap 'kill "$run_pid" >/dev/null 2>&1 || true' EXIT

for _ in {1..120}; do
  run_url="$(grep -Eo 'http://127\.0\.0\.1:[0-9]+' "$run_log" 2>/dev/null | head -n 1 || true)"
  run_url="${run_url:-http://127.0.0.1:$DEFAULT_RUN_PORT}"
  if curl --fail --silent "$run_url" >/dev/null; then
    echo "source-linked macOS smoke run passed at $SMOKE_PATH"
    exit 0
  fi
  if ! kill -0 "$run_pid" >/dev/null 2>&1; then
    cat "$run_log" >&2
    exit 1
  fi
  sleep 2
done

cat "$run_log" >&2
echo "Timed out waiting for pax-cli run web server." >&2
exit 1
GUEST_SCRIPT

if ! prlctl list -a -o name,status 2>/dev/null | awk -v vm="$VM_NAME" '$1 == vm { print $2 }' | grep -Fxq "running"; then
  prlctl start "$VM_NAME"
fi

guest_bash() {
  prlctl exec "$VM_NAME" --user "$VM_USER" --password "$VM_PASSWORD" /bin/bash -s
}

deadline=$((SECONDS + ${PAX_FIRST_TOUCH_MACOS_READY_TIMEOUT_SECONDS:-600}))
until printf 'id -un >/dev/null\n' | guest_bash >/dev/null 2>&1; do
  if (( SECONDS >= deadline )); then
    echo "Timed out waiting for macOS guest control." >&2
    exit 1
  fi
  sleep 5
done

printf 'mkdir -p %q\n' "$GUEST_TMP_DIR" | guest_bash
cat "$TMP_DIR/pax-src.tar.gz" | prlctl exec "$VM_NAME" --user "$VM_USER" --password "$VM_PASSWORD" /bin/dd "of=$GUEST_TMP_DIR/pax-src.tar.gz" bs=1048576
cat "$TMP_DIR/smoke-macos-source.sh" | prlctl exec "$VM_NAME" --user "$VM_USER" --password "$VM_PASSWORD" /bin/dd "of=$GUEST_TMP_DIR/smoke-macos-source.sh" bs=1048576

{
  printf 'chmod +x %q\n' "$GUEST_TMP_DIR/smoke-macos-source.sh"
  printf '%q %q %q %q %q\n' \
    "$GUEST_TMP_DIR/smoke-macos-source.sh" \
    "$GUEST_TMP_DIR/pax-src.tar.gz" \
    "$GUEST_SRC" \
    "$SMOKE_DIR" \
    "$DEFAULT_RUN_PORT"
} | guest_bash
