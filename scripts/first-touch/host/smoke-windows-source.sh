#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PAX_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
VM_NAME="${PAX_FIRST_TOUCH_VM_NAME:-pax-windows-first-touch}"
VM_USER="${PAX_FIRST_TOUCH_USERNAME:-pax}"
VM_PASSWORD="${PAX_FIRST_TOUCH_PASSWORD:-}"
TMP_DIR="${PAX_FIRST_TOUCH_TMP_DIR:-/tmp/pax-first-touch/windows-source-smoke}"
SHARE_NAME="${PAX_FIRST_TOUCH_SHARE_NAME:-PaxFirstTouchSmoke}"
GUEST_SRC="${PAX_FIRST_TOUCH_GUEST_SRC:-C:/Users/$VM_USER/pax-src}"
SMOKE_DIR="${PAX_FIRST_TOUCH_SMOKE_DIR:-C:/Users/$VM_USER/pax-smoke-source}"

if [[ -z "$VM_PASSWORD" ]]; then
  echo "Set PAX_FIRST_TOUCH_PASSWORD to the Windows VM password." >&2
  exit 1
fi

rm -rf "$TMP_DIR"
mkdir -p "$TMP_DIR"

COPYFILE_DISABLE=1 tar \
  --dereference \
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

cat > "$TMP_DIR/smoke-windows-source.ps1" <<'POWERSHELL'
param(
    [Parameter(Mandatory = $true)][string]$ArchivePath,
    [Parameter(Mandatory = $true)][string]$SourcePath,
    [Parameter(Mandatory = $true)][string]$SmokePath
)

$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"

function Invoke-NativeCommand {
    param(
        [Parameter(Mandatory = $true)][string]$FilePath,
        [string[]]$ArgumentList = @()
    )

    & $FilePath @ArgumentList
    if ($LASTEXITCODE -ne 0) {
        throw "$FilePath exited with $LASTEXITCODE"
    }
}

function Import-MSVCEnvironment {
    $vcvars = "C:\BuildTools\VC\Auxiliary\Build\vcvarsall.bat"
    if (-not (Test-Path $vcvars)) {
        throw "Visual Studio Build Tools vcvarsall.bat was not found at $vcvars"
    }

    $arch = if ($env:PROCESSOR_ARCHITECTURE -eq "ARM64") { "arm64" } else { "x64" }
    $environment = cmd.exe /s /c "`"$vcvars`" $arch >nul && set"
    foreach ($line in $environment) {
        $separator = $line.IndexOf("=")
        if ($separator -gt 0) {
            $name = $line.Substring(0, $separator)
            $value = $line.Substring($separator + 1)
            Set-Item -Path "Env:$name" -Value $value
        }
    }
}

$extraPath = @(
    "$env:USERPROFILE\.cargo\bin",
    "$env:USERPROFILE\.local\pax-source\bin",
    "C:\BuildTools\VC\Tools\Llvm\bin",
    "C:\Program Files\Git\cmd"
)
$env:Path = (($extraPath + @($env:Path -split ";" | Where-Object { $_ })) -join ";")
Import-MSVCEnvironment
Remove-Item -Recurse -Force -ErrorAction SilentlyContinue $SourcePath, $SmokePath
New-Item -ItemType Directory -Force -Path $SourcePath | Out-Null
Invoke-NativeCommand -FilePath "tar.exe" -ArgumentList @("-xzf", $ArchivePath, "-C", $SourcePath)

New-Item -ItemType Directory -Force -Path "$env:USERPROFILE\.local\pax-source", "$env:USERPROFILE\.cache\pax-target" | Out-Null
$env:CARGO_TARGET_DIR = "$env:USERPROFILE\.cache\pax-target\pax-cli"
Invoke-NativeCommand -FilePath "cargo.exe" -ArgumentList @("install", "--path", (Join-Path $SourcePath "pax-cli"), "--root", "$env:USERPROFILE\.local\pax-source", "--force")

Invoke-NativeCommand -FilePath "pax-cli.exe" -ArgumentList @("create", $SmokePath)
New-Item -ItemType Directory -Force -Path (Join-Path $SmokePath ".cargo") | Out-Null
$paxKitPath = (Join-Path $SourcePath "pax-kit").Replace("\", "/")
@"
[patch.crates-io]
pax-kit = { path = "$paxKitPath" }
"@ | Set-Content -Path (Join-Path $SmokePath ".cargo\config.toml") -Encoding UTF8

Push-Location $SmokePath
try {
    Invoke-NativeCommand -FilePath "pax-cli.exe" -ArgumentList @("build", "--target", "web")
}
finally {
    Pop-Location
}

$legacyMatches = Get-ChildItem -Recurse -ErrorAction SilentlyContinue `
    (Join-Path $SmokePath ".pax\interface\web"), `
    (Join-Path $SmokePath ".pax\build\debug\web") |
    Select-String -CaseSensitive -Pattern "occlusionLayer|occlusion_layer"
if ($legacyMatches) {
    $legacyMatches | Out-String | Write-Error
}

"source-linked Windows smoke build passed at $SmokePath"
POWERSHELL

cleanup() {
  prlctl set "$VM_NAME" --shf-host-del "$SHARE_NAME" >/dev/null 2>&1 || true
}
trap cleanup EXIT

if ! prlctl list -a -o name,status 2>/dev/null | awk -v vm="$VM_NAME" '$1 == vm { print $2 }' | grep -Fxq "running"; then
  prlctl start "$VM_NAME"
fi

prlctl set "$VM_NAME" --shf-host-del "$SHARE_NAME" >/dev/null 2>&1 || true
prlctl set "$VM_NAME" --shf-host-add "$SHARE_NAME" --path "$TMP_DIR" --mode ro --enable
prlctl set "$VM_NAME" --shf-host-automount on

deadline=$((SECONDS + ${PAX_FIRST_TOUCH_WINDOWS_READY_TIMEOUT_SECONDS:-600}))
until prlctl exec "$VM_NAME" --user "$VM_USER" --password "$VM_PASSWORD" powershell.exe -NoProfile -ExecutionPolicy Bypass -Command "exit 0" >/dev/null 2>&1; do
  if (( SECONDS >= deadline )); then
    echo "Timed out waiting for Windows guest control." >&2
    exit 1
  fi
  sleep 5
done

prlctl exec "$VM_NAME" --user "$VM_USER" --password "$VM_PASSWORD" powershell.exe -NoProfile -ExecutionPolicy Bypass -Command "\$ErrorActionPreference='Stop'; \$share='\\\\Mac\\$SHARE_NAME'; Copy-Item \"\$share\\pax-src.tar.gz\" \"\$env:TEMP\\pax-src.tar.gz\" -Force; Copy-Item \"\$share\\smoke-windows-source.ps1\" \"\$env:TEMP\\smoke-windows-source.ps1\" -Force; powershell.exe -NoProfile -ExecutionPolicy Bypass -File \"\$env:TEMP\smoke-windows-source.ps1\" -ArchivePath \"\$env:TEMP\pax-src.tar.gz\" -SourcePath \"$GUEST_SRC\" -SmokePath \"$SMOKE_DIR\""
