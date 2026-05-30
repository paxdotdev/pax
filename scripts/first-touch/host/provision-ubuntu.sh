#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PAX_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
PACKER_FILE="$SCRIPT_DIR/ubuntu-parallels.pkr.hcl"
VM_NAME="${PAX_FIRST_TOUCH_VM_NAME:-pax-ubuntu-first-touch}"
OUTPUT_DIRECTORY="${PAX_FIRST_TOUCH_OUTPUT_DIRECTORY:-$HOME/Parallels/${VM_NAME}.pvm}"

if ! command -v packer >/dev/null 2>&1; then
  echo "packer is required. Install it before provisioning the Ubuntu VM." >&2
  exit 1
fi

if ! command -v prlctl >/dev/null 2>&1; then
  echo "prlctl is required. Install Parallels Desktop before provisioning." >&2
  exit 1
fi

if prlctl list -a -o name 2>/dev/null | awk 'NR > 1 { print }' | grep -Fxq "$VM_NAME"; then
  echo "VM '$VM_NAME' already exists. Delete, rename, or clone it before rebuilding." >&2
  exit 1
fi

if [[ -e "$OUTPUT_DIRECTORY" ]]; then
  echo "Output directory '$OUTPUT_DIRECTORY' already exists. Move or remove it before rebuilding." >&2
  exit 1
fi

if [[ -n "${PAX_FIRST_TOUCH_PASSWORD:-}" ]]; then
  VM_PASSWORD="$PAX_FIRST_TOUCH_PASSWORD"
else
  VM_PASSWORD="$(openssl rand -base64 24 | tr -d '=+/[:space:]' | cut -c1-24)"
fi

PASSWORD_HASH="$(openssl passwd -6 "$VM_PASSWORD")"

echo "Provisioning $VM_NAME"
echo "Pax root: $PAX_ROOT"
echo "Output: $OUTPUT_DIRECTORY"
if [[ -z "${PAX_FIRST_TOUCH_PASSWORD:-}" ]]; then
  echo "Generated VM password for this run: $VM_PASSWORD"
fi

export PKR_VAR_vm_name="$VM_NAME"
export PKR_VAR_output_directory="$OUTPUT_DIRECTORY"
export PKR_VAR_ssh_password="$VM_PASSWORD"
export PKR_VAR_ssh_password_hash="$PASSWORD_HASH"

packer init "$PACKER_FILE"
packer build "$PACKER_FILE"

REGISTER_PATH="$OUTPUT_DIRECTORY"
if [[ -d "$OUTPUT_DIRECTORY/$VM_NAME.pvm" ]]; then
  REGISTER_PATH="$OUTPUT_DIRECTORY/$VM_NAME.pvm"
fi

if ! prlctl list -a -o name 2>/dev/null | awk 'NR > 1 { print }' | grep -Fxq "$VM_NAME"; then
  prlctl register "$REGISTER_PATH"
fi

SNAPSHOT_NAME="${PAX_FIRST_TOUCH_SNAPSHOT_NAME:-pax-first-touch-ubuntu-workstation-prereqs}"
if [[ -n "$SNAPSHOT_NAME" ]]; then
  prlctl snapshot "$VM_NAME" \
    --name "$SNAPSHOT_NAME" \
    --description "Ubuntu first-touch workstation prerequisites"
fi

echo "Ubuntu VM baseline created and registered at $OUTPUT_DIRECTORY"
