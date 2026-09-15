#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PAX_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
VM_NAME="${PAX_FIRST_TOUCH_VM_NAME:-pax-ubuntu-first-touch}"
GUEST_USER="${PAX_FIRST_TOUCH_GUEST_USER:-pax}"
GUEST_SRC="${PAX_FIRST_TOUCH_GUEST_SRC:-/home/pax/pax-src}"
SMOKE_DIR="${PAX_FIRST_TOUCH_SMOKE_DIR:-/home/pax/pax-smoke-source}"
GUEST_HOST="${PAX_FIRST_TOUCH_GUEST_HOST:-}"
SSH_KEY="${PAX_FIRST_TOUCH_SSH_KEY:-$HOME/.cache/pax-first-touch/keys/pax-ubuntu-first-touch_ed25519}"
KNOWN_HOSTS="${PAX_FIRST_TOUCH_KNOWN_HOSTS:-$HOME/.cache/pax-first-touch/known_hosts}"

if [[ ! "$GUEST_USER" =~ ^[A-Za-z0-9._-]+$ ]]; then
  echo "PAX_FIRST_TOUCH_GUEST_USER contains unsupported characters." >&2
  exit 1
fi

validate_guest_path() {
  local path="$1"
  local label="$2"
  if [[ ! "$path" =~ ^/[A-Za-z0-9._/-]+$ \
    || "$path" != "/home/$GUEST_USER/"* \
    || "$path" == *"/../"* \
    || "$path" == */.. \
    || "$path" == *"/./"* \
    || "$path" == */. ]]; then
    echo "$label must be a normalized descendant of /home/$GUEST_USER." >&2
    exit 1
  fi
}

validate_guest_path "$GUEST_SRC" PAX_FIRST_TOUCH_GUEST_SRC
validate_guest_path "$SMOKE_DIR" PAX_FIRST_TOUCH_SMOKE_DIR
if [[ "$GUEST_SRC" == "$SMOKE_DIR" \
  || "$GUEST_SRC" == "$SMOKE_DIR/"* \
  || "$SMOKE_DIR" == "$GUEST_SRC/"* ]]; then
  echo "Guest source and smoke paths must be separate, non-nested directories." >&2
  exit 1
fi

mkdir -p "$(dirname "$KNOWN_HOSTS")"
touch "$KNOWN_HOSTS"
chmod 600 "$KNOWN_HOSTS"

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
  -o BatchMode=yes
  -o ConnectTimeout=10
  -o StrictHostKeyChecking=accept-new
  -o ServerAliveInterval=15
  -o ServerAliveCountMax=4
  -o UserKnownHostsFile="$KNOWN_HOSTS"
)

if [[ -f "$SSH_KEY" ]]; then
  SSH_OPTS=(-i "$SSH_KEY" "${SSH_OPTS[@]}")
fi

SSH_TARGET="$GUEST_USER@$GUEST_HOST"

ssh-keygen -R "$GUEST_HOST" -f "$KNOWN_HOSTS" >/dev/null 2>&1 || true

ssh "${SSH_OPTS[@]}" "$SSH_TARGET" 'bash -s --' "$GUEST_SRC" <<'REMOTE'
set -euo pipefail
source_path="$1"
rm -rf -- "$source_path"
mkdir -p "$source_path"
REMOTE

RSYNC_RSH="ssh"
for option in "${SSH_OPTS[@]}"; do
  printf -v quoted_option '%q' "$option"
  RSYNC_RSH+=" $quoted_option"
done

git -C "$PAX_ROOT" ls-files --cached -z \
  | rsync -az --from0 --files-from=- \
  -e "$RSYNC_RSH" \
  "$PAX_ROOT/" "$SSH_TARGET:$GUEST_SRC/"

git -C "$PAX_ROOT" ls-files --others --exclude-standard -z \
  | rsync -az --from0 --files-from=- \
  --exclude '.env' \
  --exclude '.env.*' \
  --exclude '*.key' \
  --exclude '*.pem' \
  --exclude '.aws/***' \
  --exclude '.ssh/***' \
  --exclude '.npmrc' \
  --exclude '.pypirc' \
  --exclude 'credentials.json' \
  -e "$RSYNC_RSH" \
  "$PAX_ROOT/" "$SSH_TARGET:$GUEST_SRC/"

ssh "${SSH_OPTS[@]}" "$SSH_TARGET" 'bash -s --' "$GUEST_SRC" "$SMOKE_DIR" <<'REMOTE'
set -euo pipefail

GUEST_SRC="$1"
SMOKE_DIR="$2"
source "$HOME/.cargo/env"
export PATH="$HOME/.local/pax-source/bin:$PATH"

VALIDATION_DIR="$(mktemp -d /tmp/pax-ubuntu-telemetry.XXXXXX)"
CAPTURE_FILE="$VALIDATION_DIR/events.jsonl"
PORT_FILE="$VALIDATION_DIR/port"
CAPTURE_PID=""
RUN_PID=""

stop_process() {
  local pid="$1"
  kill "$pid" >/dev/null 2>&1 || true
  for _ in {1..20}; do
    kill -0 "$pid" >/dev/null 2>&1 || break
    sleep 0.05
  done
  if kill -0 "$pid" >/dev/null 2>&1; then
    kill -KILL "$pid" >/dev/null 2>&1 || true
  fi
  wait "$pid" >/dev/null 2>&1 || true
}

cleanup() {
  if [[ -n "$RUN_PID" ]]; then
    stop_process "$RUN_PID"
  fi
  if [[ -n "$CAPTURE_PID" ]]; then
    stop_process "$CAPTURE_PID"
  fi
  rm -rf "$VALIDATION_DIR"
}
trap cleanup EXIT

cat > "$VALIDATION_DIR/capture.py" <<'PYTHON'
import json
import pathlib
import sys
from http.server import BaseHTTPRequestHandler, HTTPServer

port_file = pathlib.Path(sys.argv[1])
capture_file = pathlib.Path(sys.argv[2])


class Handler(BaseHTTPRequestHandler):
    def do_GET(self):
        if self.path != "/v1/cli/releases/latest":
            self.send_error(404)
            return
        body = json.dumps({"latest_version": "0.0.0"}).encode()
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def do_POST(self):
        if self.path != "/v1/cli/telemetry":
            self.send_error(404)
            return
        length = int(self.headers.get("Content-Length", "0"))
        body = self.rfile.read(length)
        with capture_file.open("ab") as capture:
            capture.write(body + b"\n")
        self.send_response(204)
        self.end_headers()

    def log_message(self, _format, *_args):
        pass


server = HTTPServer(("127.0.0.1", 0), Handler)
port_file.write_text(str(server.server_port))
server.serve_forever()
PYTHON

python3 "$VALIDATION_DIR/capture.py" "$PORT_FILE" "$CAPTURE_FILE" &
CAPTURE_PID=$!
for _ in {1..100}; do
  [[ -s "$PORT_FILE" ]] && break
  kill -0 "$CAPTURE_PID" >/dev/null 2>&1
  sleep 0.05
done
[[ -s "$PORT_FILE" ]]

export PAX_API_BASE_URL="http://127.0.0.1:$(cat "$PORT_FILE")"
export XDG_STATE_HOME="$VALIDATION_DIR/state"
unset CI DO_NOT_TRACK PAX_TELEMETRY

mkdir -p "$HOME/.local/pax-source" "$HOME/.cache/pax-target"
CARGO_TARGET_DIR="$HOME/.cache/pax-target/pax-cli" \
  cargo install --path "$GUEST_SRC/pax-cli" --root "$HOME/.local/pax-source" --force

rm -rf "$SMOKE_DIR"
CREATE_LOG="$VALIDATION_DIR/create.log"
pax-cli create "$SMOKE_DIR" >"$CREATE_LOG" 2>&1
grep -Fq "This command sends no telemetry." "$CREATE_LOG"
[[ ! -s "$CAPTURE_FILE" ]]

STATE_DIR="$XDG_STATE_HOME/pax/telemetry"
STATE_FILE="$STATE_DIR/installation.json"
[[ "$(stat -c '%a' "$STATE_DIR")" == "700" ]]
[[ "$(stat -c '%a' "$STATE_FILE")" == "600" ]]
python3 - "$STATE_FILE" <<'PYTHON'
import json
import pathlib
import sys
import uuid

state = json.loads(pathlib.Path(sys.argv[1]).read_text())
assert set(state) == {"schema_version", "notice_version", "installation_id"}
assert state["schema_version"] == 1
assert state["notice_version"] == 1
installation_id = uuid.UUID(state["installation_id"])
assert installation_id.version == 4
PYTHON

mkdir -p "$SMOKE_DIR/.cargo"
cat > "$SMOKE_DIR/.cargo/config.toml" <<EOF
[patch.crates-io]
pax-kit = { path = "$GUEST_SRC/pax-kit" }
EOF

cd "$SMOKE_DIR"
BUILD_LOG="$VALIDATION_DIR/build.log"
pax-cli build --target web >"$BUILD_LOG" 2>&1
if grep -Fq "This command sends no telemetry." "$BUILD_LOG"; then
  echo "The telemetry notice repeated on the second public command." >&2
  exit 1
fi

for _ in {1..100}; do
  [[ -f "$CAPTURE_FILE" ]] && [[ "$(wc -l < "$CAPTURE_FILE")" -ge 1 ]] && break
  sleep 0.05
done
[[ "$(wc -l < "$CAPTURE_FILE")" == "1" ]]

python3 - "$STATE_FILE" "$CAPTURE_FILE" "$SMOKE_DIR" <<'PYTHON'
import json
import pathlib
import sys

state = json.loads(pathlib.Path(sys.argv[1]).read_text())
raw = pathlib.Path(sys.argv[2]).read_text()
events = [json.loads(line) for line in raw.splitlines()]
assert len(events) == 1
request = events[0]
assert set(request) == {"installation_id", "cli_version", "host_os", "host_arch", "event"}
assert request["installation_id"] == state["installation_id"]
assert request["host_os"] == "linux"
assert request["host_arch"] == "aarch64"
assert set(request["event"]) == {"type", "command", "target", "outcome"}
assert request["event"] == {
    "type": "command_outcome",
    "command": "build",
    "target": "web",
    "outcome": "succeeded",
}
assert sys.argv[3] not in raw
PYTHON

if grep -RIn "occlusionLayer\\|occlusion_layer" .pax/interface/web .pax/build/debug/web 2>/dev/null; then
  echo "Legacy occlusion layer symbols leaked into the generated web interface." >&2
  exit 1
fi

RUN_LOG="$VALIDATION_DIR/run.log"
pax-cli run --target web >"$RUN_LOG" 2>&1 &
RUN_PID=$!
for _ in {1..120}; do
  RUN_URL="$(grep -Eo 'http://127\.0\.0\.1:[0-9]+' "$RUN_LOG" 2>/dev/null | head -n 1 || true)"
  RUN_URL="${RUN_URL:-http://127.0.0.1:8080}"
  if curl --fail --silent --connect-timeout 1 --max-time 2 "$RUN_URL" >/dev/null; then
    break
  fi
  if ! kill -0 "$RUN_PID" >/dev/null 2>&1; then
    cat "$RUN_LOG" >&2
    exit 1
  fi
  sleep 2
done
curl --fail --silent --connect-timeout 1 --max-time 2 "$RUN_URL" >/dev/null

for _ in {1..100}; do
  [[ "$(wc -l < "$CAPTURE_FILE")" -ge 2 ]] && break
  sleep 0.05
done
[[ "$(wc -l < "$CAPTURE_FILE")" == "2" ]]
python3 - "$STATE_FILE" "$CAPTURE_FILE" <<'PYTHON'
import json
import pathlib
import sys

state = json.loads(pathlib.Path(sys.argv[1]).read_text())
events = [json.loads(line) for line in pathlib.Path(sys.argv[2]).read_text().splitlines()]
request = events[1]
assert set(request) == {"installation_id", "cli_version", "host_os", "host_arch", "event"}
assert request["installation_id"] == state["installation_id"]
assert request["host_os"] == "linux"
assert request["host_arch"] == "aarch64"
assert request["event"] == {"type": "run_ready", "target": "web"}
PYTHON

stop_process "$RUN_PID"
RUN_PID=""

pax-cli telemetry off >/dev/null
[[ ! -e "$STATE_FILE" ]]
[[ -f "$STATE_DIR/disabled" ]]
pax-cli clean --path "$SMOKE_DIR" >/dev/null
[[ "$(wc -l < "$CAPTURE_FILE")" == "2" ]]

pax-cli telemetry on >/dev/null
CI_STATUS="$(CI=1 pax-cli telemetry status)"
[[ "$CI_STATUS" == *"CI environment"* ]]
CI=1 pax-cli clean --path "$SMOKE_DIR" >/dev/null
[[ "$(wc -l < "$CAPTURE_FILE")" == "2" ]]

PAX_API_BASE_URL=http://127.0.0.1:1 python3 - <<'PYTHON'
import subprocess

subprocess.run(
    ["pax-cli", "clean", "--path", "."],
    check=True,
    stdout=subprocess.DEVNULL,
    stderr=subprocess.DEVNULL,
    timeout=1,
)
PYTHON
pax-cli telemetry off >/dev/null

printf 'source-linked Ubuntu telemetry/create/build/run smoke passed at %s\n' "$SMOKE_DIR"
REMOTE
