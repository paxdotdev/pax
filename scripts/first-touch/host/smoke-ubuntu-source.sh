#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PAX_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
VM_NAME="${PAX_FIRST_TOUCH_VM_NAME:-pax-ubuntu-first-touch}"
GUEST_USER="${PAX_FIRST_TOUCH_GUEST_USER:-pax}"
GUEST_SRC="${PAX_FIRST_TOUCH_GUEST_SRC:-/home/pax/pax-src}"
SMOKE_DIR="${PAX_FIRST_TOUCH_SMOKE_DIR:-/home/pax/pax-smoke-source}"
GUEST_HOST="${PAX_FIRST_TOUCH_GUEST_HOST:-}"
SSH_KEY="${PAX_FIRST_TOUCH_SSH_KEY:-/tmp/pax-first-touch/pax_vm_ed25519}"
KNOWN_HOSTS="${PAX_FIRST_TOUCH_KNOWN_HOSTS:-/tmp/pax-first-touch/known_hosts}"

if [[ -z "$GUEST_HOST" ]]; then
  GUEST_HOST="$(
    prlctl list -i "$VM_NAME" 2>/dev/null \
      | awk -F': ' '/IP Addresses/ { split($2, ips, ","); print ips[1]; exit }'
  )"
fi

if [[ -z "$GUEST_HOST" ]]; then
  echo "Unable to resolve an IP address for VM '$VM_NAME'." >&2
  exit 1
fi

SSH_OPTS=(
  -o StrictHostKeyChecking=no
  -o ServerAliveInterval=15
  -o ServerAliveCountMax=4
  -o UserKnownHostsFile="$KNOWN_HOSTS"
)

if [[ -f "$SSH_KEY" ]]; then
  SSH_OPTS=(-i "$SSH_KEY" "${SSH_OPTS[@]}")
fi

SSH_TARGET="$GUEST_USER@$GUEST_HOST"

rsync -az --delete \
  --exclude '.git' \
  --exclude 'target' \
  --exclude '*/target' \
  --exclude 'node_modules' \
  --exclude '.DS_Store' \
  -e "ssh ${SSH_OPTS[*]}" \
  "$PAX_ROOT/" "$SSH_TARGET:$GUEST_SRC/"

ssh "${SSH_OPTS[@]}" "$SSH_TARGET" 'bash -s' <<REMOTE
set -euo pipefail
source "\$HOME/.cargo/env"
export PATH="\$HOME/.local/pax-source/bin:\$PATH"

mkdir -p "\$HOME/.local/pax-source" "\$HOME/.cache/pax-target"
CARGO_TARGET_DIR="\$HOME/.cache/pax-target/pax-cli" \
  cargo install --path "$GUEST_SRC/pax-cli" --root "\$HOME/.local/pax-source" --force

rm -rf "$SMOKE_DIR"
pax-cli create "$SMOKE_DIR"
mkdir -p "$SMOKE_DIR/.cargo"
cat > "$SMOKE_DIR/.cargo/config.toml" <<'EOF'
[patch.crates-io]
pax-kit = { path = "$GUEST_SRC/pax-kit" }
EOF

cd "$SMOKE_DIR"
pax-cli build --target web

if grep -RIn "occlusionLayer\\|occlusion_layer" .pax/interface/web .pax/build/debug/web 2>/dev/null; then
  echo "Legacy occlusion layer symbols leaked into the generated web interface." >&2
  exit 1
fi

printf 'source-linked Ubuntu smoke build passed at %s\\n' "$SMOKE_DIR"
REMOTE
