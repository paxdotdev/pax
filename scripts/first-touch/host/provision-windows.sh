#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PAX_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
VM_NAME="${PAX_FIRST_TOUCH_VM_NAME:-pax-windows-first-touch}"
VM_USER="${PAX_FIRST_TOUCH_USERNAME:-pax}"
COMPUTER_NAME="${PAX_FIRST_TOUCH_WINDOWS_COMPUTER_NAME:-pax-win-touch}"
PRODUCT_KEY="${PAX_FIRST_TOUCH_WINDOWS_PRODUCT_KEY:-VK7JG-NPHTM-C97JM-9MPGT-3V66T}"
OUTPUT_DIRECTORY="${PAX_FIRST_TOUCH_OUTPUT_DIRECTORY:-$HOME/Parallels/${VM_NAME}.pvm}"
WINDOWS_ISO="${PAX_FIRST_TOUCH_WINDOWS_ISO:-$HOME/.cache/pax-first-touch/iso/Win11_25H2_English_Arm64_v2.iso}"
WINDOWS_ISO_SHA256="${PAX_FIRST_TOUCH_WINDOWS_ISO_SHA256:-638AA2C88E94385B00F4F178D071E3DF0B7D9E335577A83BD533B7F2EB65ADF0}"
DISK_SIZE_MB="${PAX_FIRST_TOUCH_DISK_SIZE_MB:-131072}"
MEMORY_MB="${PAX_FIRST_TOUCH_MEMORY_MB:-8192}"
CPUS="${PAX_FIRST_TOUCH_CPUS:-4}"
TIMEZONE="${PAX_FIRST_TOUCH_WINDOWS_TIMEZONE:-Pacific Standard Time}"
GENERATED_DIR="${PAX_FIRST_TOUCH_GENERATED_DIR:-$HOME/.cache/pax-first-touch/generated/$VM_NAME}"
TOOLS_DIR="/Applications/Parallels Desktop.app/Contents/Resources/Tools"

if ! command -v prlctl >/dev/null 2>&1; then
  echo "prlctl is required. Install Parallels Desktop before provisioning." >&2
  exit 1
fi

if ! command -v hdiutil >/dev/null 2>&1; then
  echo "hdiutil is required to create the Windows automation ISO." >&2
  exit 1
fi

if [[ ! -f "$WINDOWS_ISO" ]]; then
  cat >&2 <<EOF
Windows ISO not found at:
  $WINDOWS_ISO

Download the Windows 11 Arm64 ISO from Microsoft and set
PAX_FIRST_TOUCH_WINDOWS_ISO to its path, or place it at the default path above.
EOF
  exit 1
fi

if [[ "$WINDOWS_ISO_SHA256" != "skip" ]]; then
  actual_hash="$(shasum -a 256 "$WINDOWS_ISO" | awk '{ print toupper($1) }')"
  if [[ "$actual_hash" != "$WINDOWS_ISO_SHA256" ]]; then
    echo "Windows ISO SHA256 mismatch." >&2
    echo "Expected: $WINDOWS_ISO_SHA256" >&2
    echo "Actual:   $actual_hash" >&2
    exit 1
  fi
fi

if [[ ! "$VM_USER" =~ ^[A-Za-z0-9_.-]+$ ]]; then
  echo "PAX_FIRST_TOUCH_USERNAME may only contain letters, numbers, underscore, dot, and dash." >&2
  exit 1
fi

if [[ ! "$COMPUTER_NAME" =~ ^[A-Za-z0-9-]+$ || "${#COMPUTER_NAME}" -gt 15 ]]; then
  echo "PAX_FIRST_TOUCH_WINDOWS_COMPUTER_NAME must be 15 characters or fewer and contain only letters, numbers, and dash." >&2
  exit 1
fi

if [[ ! "$PRODUCT_KEY" =~ ^[A-Za-z0-9-]{29}$ ]]; then
  echo "PAX_FIRST_TOUCH_WINDOWS_PRODUCT_KEY must look like a Windows product key." >&2
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

if prlctl list -a -o name 2>/dev/null | awk 'NR > 1 { print }' | grep -Fxq "$VM_NAME"; then
  echo "VM '$VM_NAME' already exists. Delete, rename, or clone it before rebuilding." >&2
  exit 1
fi

if [[ -e "$OUTPUT_DIRECTORY" ]]; then
  echo "Output directory '$OUTPUT_DIRECTORY' already exists. Move or remove it before rebuilding." >&2
  exit 1
fi

mkdir -p "$GENERATED_DIR/files"
rm -rf "$GENERATED_DIR/files"
mkdir -p "$GENERATED_DIR/files"

sed \
  -e "s|@@USERNAME@@|$VM_USER|g" \
  -e "s|@@PASSWORD@@|$VM_PASSWORD|g" \
  -e "s|@@COMPUTERNAME@@|$COMPUTER_NAME|g" \
  -e "s|@@PRODUCTKEY@@|$PRODUCT_KEY|g" \
  -e "s|@@TIMEZONE@@|$TIMEZONE|g" \
  "$SCRIPT_DIR/../guest/windows-autounattend.xml.pkrtpl" \
  > "$GENERATED_DIR/files/Autounattend.xml"

cp "$SCRIPT_DIR/../guest/windows-workstation-prereqs.ps1" "$GENERATED_DIR/files/windows-workstation-prereqs.ps1"
cp "$TOOLS_DIR/igt_arm64.exe" "$GENERATED_DIR/files/igt_arm64.exe"
cp -R "$TOOLS_DIR/prl_tg" "$GENERATED_DIR/files/prl_tg"
cp -R "$TOOLS_DIR/netkvm" "$GENERATED_DIR/files/netkvm"

rm -f "$GENERATED_DIR/pax-windows-setup.iso"
hdiutil makehybrid \
  -iso \
  -joliet \
  -default-volume-name PAXWINSETUP \
  -o "$GENERATED_DIR/pax-windows-setup.iso" \
  "$GENERATED_DIR/files" >/dev/null

echo "Provisioning $VM_NAME"
echo "Pax root: $PAX_ROOT"
echo "Output: $OUTPUT_DIRECTORY"
echo "Windows ISO: $WINDOWS_ISO"
echo "Automation ISO: $GENERATED_DIR/pax-windows-setup.iso"
if [[ -z "${PAX_FIRST_TOUCH_PASSWORD:-}" ]]; then
  echo "Generated VM password for this run: $VM_PASSWORD"
fi

start_windows_prereq_task() {
  echo "Starting Windows prerequisite task through Parallels guest control..."
  prlctl set "$VM_NAME" --device-connect cdrom1 >/dev/null 2>&1 || true
  {
    printf '%s\r\n' '@echo off'
    printf '%s\r\n' 'if exist C:\pax-first-touch\workstation-prereqs.done exit /b 0'
    printf '%s\r\n' 'mkdir C:\pax-first-touch 2>nul'
    printf '%s\r\n' 'for %I in (D E F G H I J K L M N O P Q R S T U V W X Y Z) do if exist %I:\windows-workstation-prereqs.ps1 copy /Y %I:\windows-workstation-prereqs.ps1 C:\pax-first-touch\windows-workstation-prereqs.ps1 >nul'
    printf '%s\r\n' 'if not exist C:\pax-first-touch\windows-workstation-prereqs.ps1 exit /b 1'
    printf '%s\r\n' "schtasks /Query /TN PaxFirstTouchPrereqs >nul 2>nul || schtasks /Create /TN PaxFirstTouchPrereqs /SC ONCE /ST 23:59 /TR \"powershell.exe -NoProfile -ExecutionPolicy Bypass -File C:\\pax-first-touch\\windows-workstation-prereqs.ps1\" /RU $VM_USER /RP $VM_PASSWORD /RL HIGHEST /F >nul"
    printf '%s\r\n' 'schtasks /Run /TN PaxFirstTouchPrereqs >nul'
    printf '%s\r\n' 'exit'
  } | prlctl enter "$VM_NAME" >/dev/null
}

prlctl create "$VM_NAME" --distribution win-11 --no-hdd --dst "$(dirname "$OUTPUT_DIRECTORY")"
prlctl set "$VM_NAME" \
  --memsize "$MEMORY_MB" \
  --cpus "$CPUS" \
  --startup-view headless \
  --on-window-close keep-running \
  --shared-profile off \
  --sh-app-host-to-guest off \
  --sh-app-guest-to-host off \
  --shared-cloud off
prlctl set "$VM_NAME" --device-del sound0 >/dev/null 2>&1 || true
prlctl set "$VM_NAME" --device-add hdd --type expand --size "$DISK_SIZE_MB" --iface nvme
prlctl set "$VM_NAME" --device-set cdrom0 --image "$WINDOWS_ISO" --connect
prlctl set "$VM_NAME" --device-add cdrom --image "$GENERATED_DIR/pax-windows-setup.iso" --connect --iface sata
prlctl set "$VM_NAME" --device-bootorder "cdrom0 hdd0"

prlctl start "$VM_NAME"

deadline=$((SECONDS + ${PAX_FIRST_TOUCH_WINDOWS_TIMEOUT_SECONDS:-14400}))
prereq_task_started=0
echo "Waiting for Windows first-logon prerequisite marker..."
while (( SECONDS < deadline )); do
  set +e
  prlctl exec "$VM_NAME" --user "$VM_USER" --password "$VM_PASSWORD" powershell.exe -NoProfile -ExecutionPolicy Bypass -Command "if (Test-Path 'C:\pax-first-touch\workstation-prereqs.done') { exit 0 } elseif (Test-Path 'C:\pax-first-touch\workstation-prereqs.failed') { exit 2 } else { exit 1 }" >/dev/null 2>&1
  status=$?
  set -e

  if [[ "$status" == "0" ]]; then
    break
  fi

  if [[ "$status" == "2" ]]; then
    echo "Windows prerequisite script failed. Check C:\\pax-first-touch\\workstation-prereqs.log inside the VM." >&2
    exit 1
  fi

  if [[ "$status" == "1" && "$prereq_task_started" == "0" ]]; then
    start_windows_prereq_task
    prereq_task_started=1
  fi

  sleep 30
done

if (( SECONDS >= deadline )); then
  echo "Timed out waiting for Windows prerequisite marker." >&2
  exit 1
fi

prlctl exec "$VM_NAME" --user "$VM_USER" --password "$VM_PASSWORD" powershell.exe -NoProfile -ExecutionPolicy Bypass -Command "Stop-Computer -Force" || true

for _ in {1..120}; do
  if prlctl list -a -o name,status 2>/dev/null | awk -v vm="$VM_NAME" '$1 == vm { print $2 }' | grep -Fxq "stopped"; then
    break
  fi
  sleep 5
done

SNAPSHOT_NAME="${PAX_FIRST_TOUCH_SNAPSHOT_NAME:-pax-first-touch-windows-workstation-prereqs}"
if [[ -n "$SNAPSHOT_NAME" ]]; then
  prlctl snapshot "$VM_NAME" \
    --name "$SNAPSHOT_NAME" \
    --description "Windows first-touch workstation prerequisites"
fi

echo "Windows VM baseline created and registered at $OUTPUT_DIRECTORY"
