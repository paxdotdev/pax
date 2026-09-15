#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PAX_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
PACKER_FILE="$SCRIPT_DIR/ubuntu-parallels.pkr.hcl"
VM_NAME="${PAX_FIRST_TOUCH_VM_NAME:-pax-ubuntu-first-touch}"
OUTPUT_DIRECTORY="${PAX_FIRST_TOUCH_OUTPUT_DIRECTORY:-$HOME/Parallels/${VM_NAME}.pvm}"
SSH_KEY="${PAX_FIRST_TOUCH_SSH_KEY:-$HOME/.cache/pax-first-touch/keys/pax-ubuntu-first-touch_ed25519}"
SSH_PUBLIC_KEY="${PAX_FIRST_TOUCH_SSH_PUBLIC_KEY:-${SSH_KEY}.pub}"
PASSWORD_FILE="${PAX_FIRST_TOUCH_PASSWORD_FILE:-$HOME/.cache/pax-first-touch/keys/${VM_NAME}.password}"

if ! command -v packer >/dev/null 2>&1; then
  echo "packer is required. Install it before provisioning the Ubuntu VM." >&2
  exit 1
fi

if ! command -v prlctl >/dev/null 2>&1; then
  echo "prlctl is required. Install Parallels Desktop before provisioning." >&2
  exit 1
fi

if ! command -v ssh-keygen >/dev/null 2>&1; then
  echo "ssh-keygen is required to provision durable Ubuntu VM access." >&2
  exit 1
fi

if [[ ! -f "$SSH_PUBLIC_KEY" ]]; then
  if [[ ! -f "$SSH_KEY" ]]; then
    if [[ -n "${PAX_FIRST_TOUCH_SSH_PUBLIC_KEY:-}" ]]; then
      echo "Configured Ubuntu SSH public key '$SSH_PUBLIC_KEY' does not exist." >&2
      exit 1
    fi
    mkdir -p "$(dirname "$SSH_KEY")"
    chmod 700 "$(dirname "$SSH_KEY")"
    ssh-keygen -q -t ed25519 -N '' -C pax-first-touch -f "$SSH_KEY"
  else
    mkdir -p "$(dirname "$SSH_PUBLIC_KEY")"
    ssh-keygen -y -f "$SSH_KEY" > "$SSH_PUBLIC_KEY"
    chmod 644 "$SSH_PUBLIC_KEY"
  fi
fi

SSH_AUTHORIZED_KEY="$(tr -d '\r\n' < "$SSH_PUBLIC_KEY")"
if [[ "$SSH_AUTHORIZED_KEY" != ssh-* ]]; then
  echo "Ubuntu SSH public key '$SSH_PUBLIC_KEY' is not an OpenSSH public key." >&2
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

if [[ ! "$VM_PASSWORD" =~ ^[A-Za-z0-9_.-]+$ ]]; then
  echo "PAX_FIRST_TOUCH_PASSWORD may only contain letters, numbers, underscore, dot, and dash." >&2
  exit 1
fi

PASSWORD_HASH="$(printf '%s\n' "$VM_PASSWORD" | openssl passwd -6 -stdin)"

echo "Provisioning $VM_NAME"
echo "Pax root: $PAX_ROOT"
echo "Output: $OUTPUT_DIRECTORY"
if [[ -z "${PAX_FIRST_TOUCH_PASSWORD:-}" ]]; then
  mkdir -p "$(dirname "$PASSWORD_FILE")"
  umask 077
  printf '%s\n' "$VM_PASSWORD" > "$PASSWORD_FILE"
  chmod 600 "$PASSWORD_FILE"
  echo "Generated VM password stored at $PASSWORD_FILE"
fi

export PKR_VAR_vm_name="$VM_NAME"
export PKR_VAR_output_directory="$OUTPUT_DIRECTORY"
export PKR_VAR_ssh_password="$VM_PASSWORD"
export PKR_VAR_ssh_password_hash="$PASSWORD_HASH"
export PKR_VAR_ssh_authorized_key="$SSH_AUTHORIZED_KEY"

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
