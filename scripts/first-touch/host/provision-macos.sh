#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PAX_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
VM_NAME="${PAX_FIRST_TOUCH_VM_NAME:-pax-macos-first-touch}"
VM_USER="${PAX_FIRST_TOUCH_USERNAME:-pax}"
OUTPUT_DIRECTORY="${PAX_FIRST_TOUCH_OUTPUT_DIRECTORY:-$HOME/Parallels/${VM_NAME}.macvm}"
IPSW_CACHE_DIR="${PAX_FIRST_TOUCH_IPSW_CACHE_DIR:-$HOME/.cache/pax-first-touch/ipsw}"
MACOS_IPSW="${PAX_FIRST_TOUCH_MACOS_IPSW:-}"
MACOS_IPSW_CATALOG_URL="${PAX_FIRST_TOUCH_MACOS_IPSW_CATALOG_URL:-https://mesu.apple.com/assets/macos/com_apple_macOSIPSW/com_apple_macOSIPSW.xml}"
CPUS="${PAX_FIRST_TOUCH_CPUS:-4}"
MEMORY_MB="${PAX_FIRST_TOUCH_MEMORY_MB:-8192}"
REUSE_EXISTING="${PAX_FIRST_TOUCH_REUSE_EXISTING:-0}"

if ! command -v prlctl >/dev/null 2>&1; then
  echo "prlctl is required. Install Parallels Desktop before provisioning." >&2
  exit 1
fi

if ! command -v python3 >/dev/null 2>&1; then
  echo "python3 is required to read Apple's macOS IPSW catalog." >&2
  exit 1
fi

if ! command -v curl >/dev/null 2>&1; then
  echo "curl is required to download the macOS restore image." >&2
  exit 1
fi

if [[ -n "${PAX_FIRST_TOUCH_PASSWORD:-}" ]]; then
  VM_PASSWORD="$PAX_FIRST_TOUCH_PASSWORD"
else
  VM_PASSWORD="$(openssl rand -base64 24 | tr -d '=+/[:space:]' | cut -c1-24)"
fi

if [[ ! "$VM_USER" =~ ^[A-Za-z0-9_.-]+$ ]]; then
  echo "PAX_FIRST_TOUCH_USERNAME may only contain letters, numbers, underscore, dot, and dash." >&2
  exit 1
fi

if [[ ! "$VM_PASSWORD" =~ ^[A-Za-z0-9_.-]+$ ]]; then
  echo "PAX_FIRST_TOUCH_PASSWORD may only contain letters, numbers, underscore, dot, and dash." >&2
  exit 1
fi

VM_EXISTS=0
if prlctl list -a -o name 2>/dev/null | awk 'NR > 1 { print }' | grep -Fxq "$VM_NAME"; then
  if [[ "$REUSE_EXISTING" != "1" ]]; then
    echo "VM '$VM_NAME' already exists. Delete, rename, clone it, or set PAX_FIRST_TOUCH_REUSE_EXISTING=1 to continue provisioning it." >&2
    exit 1
  fi
  VM_EXISTS=1
fi

if [[ "$VM_EXISTS" == "0" && -e "$OUTPUT_DIRECTORY" ]]; then
  echo "Output directory '$OUTPUT_DIRECTORY' already exists. Move or remove it before rebuilding." >&2
  exit 1
fi

resolve_restore_image() {
  python3 - "$MACOS_IPSW_CATALOG_URL" <<'PY'
import plistlib
import subprocess
import sys
import urllib.request

catalog_url = sys.argv[1]
host_model = subprocess.check_output(["sysctl", "-n", "hw.model"], text=True).strip()
with urllib.request.urlopen(catalog_url, timeout=60) as response:
    catalog = plistlib.loads(response.read())

versions = catalog["MobileDeviceSoftwareVersionsByVersion"]["1"]["MobileDeviceSoftwareVersions"]

def restore_entries_for(model):
    entries = []

    def walk(value):
        if isinstance(value, dict):
            restore = value.get("Restore")
            if isinstance(restore, dict) and restore.get("FirmwareURL"):
                entries.append(restore)
            for child in value.values():
                walk(child)

    walk(versions.get(model, {}))
    return entries

entries = restore_entries_for("VirtualMac2,1") or restore_entries_for(host_model)
if not entries:
    raise SystemExit(f"no macOS IPSW restore image found for VirtualMac2,1 or {host_model}")

restore = entries[0]
url = restore["FirmwareURL"]
sha1 = restore.get("FirmwareSHA1", "")
if not sha1:
    for model_data in versions.values():
        stack = [model_data]
        while stack:
            item = stack.pop()
            if not isinstance(item, dict):
                continue
            candidate = item.get("Restore")
            if isinstance(candidate, dict) and candidate.get("FirmwareURL") == url:
                sha1 = candidate.get("FirmwareSHA1", sha1)
            stack.extend(item.values())

print("\t".join([
    restore.get("ProductVersion", ""),
    restore.get("BuildVersion", ""),
    url,
    sha1,
]))
PY
}

if [[ "$VM_EXISTS" == "0" && -z "$MACOS_IPSW" ]]; then
  IFS=$'\t' read -r PRODUCT_VERSION BUILD_VERSION RESTORE_URL RESTORE_SHA1 < <(resolve_restore_image)
  mkdir -p "$IPSW_CACHE_DIR"
  MACOS_IPSW="$IPSW_CACHE_DIR/$(basename "$RESTORE_URL")"
elif [[ "$VM_EXISTS" == "0" ]]; then
  PRODUCT_VERSION="${PAX_FIRST_TOUCH_MACOS_VERSION:-unknown}"
  BUILD_VERSION="${PAX_FIRST_TOUCH_MACOS_BUILD:-unknown}"
  RESTORE_URL=""
  RESTORE_SHA1="${PAX_FIRST_TOUCH_MACOS_IPSW_SHA1:-}"
fi

if [[ "$VM_EXISTS" == "0" && ! -f "$MACOS_IPSW" ]]; then
  if [[ -z "$RESTORE_URL" ]]; then
    echo "PAX_FIRST_TOUCH_MACOS_IPSW points to a missing file and no download URL was resolved." >&2
    exit 1
  fi
  echo "Downloading macOS $PRODUCT_VERSION ($BUILD_VERSION) restore image from Apple..."
  curl -L --fail --continue-at - --output "$MACOS_IPSW" "$RESTORE_URL"
fi

if [[ "$VM_EXISTS" == "0" && -n "${RESTORE_SHA1:-}" ]]; then
  actual_sha1="$(shasum -a 1 "$MACOS_IPSW" | awk '{ print $1 }')"
  if [[ "$actual_sha1" != "$RESTORE_SHA1" ]]; then
    echo "macOS restore image SHA1 mismatch." >&2
    echo "Expected: $RESTORE_SHA1" >&2
    echo "Actual:   $actual_sha1" >&2
    exit 1
  fi
fi

echo "Provisioning $VM_NAME"
echo "Pax root: $PAX_ROOT"
echo "Output: $OUTPUT_DIRECTORY"
if [[ "$VM_EXISTS" == "0" ]]; then
  echo "macOS restore image: $MACOS_IPSW"
else
  echo "Reusing existing VM: $VM_NAME"
fi
if [[ -z "${PAX_FIRST_TOUCH_PASSWORD:-}" ]]; then
  echo "Generated VM password for this run: $VM_PASSWORD"
  echo "Use this password if macOS Setup Assistant asks you to create the '$VM_USER' account."
fi

if [[ "$VM_EXISTS" == "0" ]]; then
  prlctl create "$VM_NAME" -o macos --restore-image "$MACOS_IPSW" --dst "$(dirname "$OUTPUT_DIRECTORY")"
  prlctl set "$VM_NAME" \
    --memsize "$MEMORY_MB" \
    --cpus "$CPUS" \
    --startup-view headless \
    --on-window-close keep-running \
    --shared-profile off \
    --sh-app-host-to-guest off \
    --sh-app-guest-to-host off \
    --shared-cloud off
  prlctl set "$VM_NAME" --shf-host-defined off >/dev/null 2>&1 || true
  prlctl set "$VM_NAME" --tools-autoupdate yes >/dev/null 2>&1 || true
fi

if ! prlctl list -a -o name,status 2>/dev/null | awk -v vm="$VM_NAME" '$1 == vm { print $2 }' | grep -Fxq "running"; then
  prlctl start "$VM_NAME"
fi

guest_bash() {
  prlctl exec "$VM_NAME" --user "$VM_USER" --password "$VM_PASSWORD" /bin/bash -s
}

deadline=$((SECONDS + ${PAX_FIRST_TOUCH_MACOS_READY_TIMEOUT_SECONDS:-7200}))
echo "Waiting for macOS guest control as user '$VM_USER'..."
until printf 'id -un >/dev/null\n' | guest_bash >/dev/null 2>&1; do
  if (( SECONDS >= deadline )); then
    cat >&2 <<EOF
Timed out waiting for macOS guest control.

If the VM is at macOS Setup Assistant, open the Parallels console, create the
local admin user '$VM_USER' with the password selected for this run, and rerun
this script with the same PAX_FIRST_TOUCH_PASSWORD after deleting or reusing the
partially-created VM as appropriate.
EOF
    exit 1
  fi
  sleep 30
done

{
  printf 'export PAX_FIRST_TOUCH_PASSWORD=%q\n' "$VM_PASSWORD"
  cat "$SCRIPT_DIR/../guest/macos-workstation-prereqs.sh"
} | guest_bash

printf 'cat "$HOME/Library/Logs/pax-first-touch/workstation-prereqs.done"\n' | guest_bash || true
printf "osascript -e 'tell app \"System Events\" to shut down'\n" | guest_bash || prlctl stop "$VM_NAME" || true

for _ in {1..120}; do
  if prlctl list -a -o name,status 2>/dev/null | awk -v vm="$VM_NAME" '$1 == vm { print $2 }' | grep -Fxq "stopped"; then
    break
  fi
  sleep 5
done

SNAPSHOT_NAME="${PAX_FIRST_TOUCH_SNAPSHOT_NAME:-pax-first-touch-macos-workstation-prereqs}"
if [[ -n "$SNAPSHOT_NAME" ]]; then
  prlctl snapshot "$VM_NAME" \
    --name "$SNAPSHOT_NAME" \
    --description "macOS first-touch workstation prerequisites"
fi

echo "macOS VM baseline created and registered at $OUTPUT_DIRECTORY"
