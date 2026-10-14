param(
    [ValidateSet('ARM64', 'x64')][string]$Architecture = 'ARM64',
    [ValidateSet('Debug', 'Release')][string]$Configuration = 'Debug',
    [string]$Cargo = 'cargo'
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
# Invoke from a matching MSVC developer shell. The explicit Cargo parameter also
# permits SYSTEM-driven builds with the interactive user's installed toolchain.
$target = if ($Architecture -eq 'ARM64') { 'aarch64-pc-windows-msvc' } else { 'x86_64-pc-windows-msvc' }
$manifest = Join-Path $PSScriptRoot 'wgpu-export\Cargo.toml'
$cargoArguments = @('build', '--locked', '--manifest-path', $manifest, '--target', $target, '-j', '2')
$profile = 'debug'
if ($Configuration -eq 'Release') { $cargoArguments += '--release'; $profile = 'release' }
& $Cargo @cargoArguments
if ($LASTEXITCODE -ne 0) { throw "Pax WGPU bridge build failed ($LASTEXITCODE)." }
$targetDirectory = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { Join-Path $PSScriptRoot 'wgpu-export\target' }
$library = Join-Path $targetDirectory "$target\$profile\pax_windows_wgpu_export_probe.dll"
$destination = Join-Path $PSScriptRoot "out\$Architecture\$Configuration"
New-Item -ItemType Directory -Force $destination | Out-Null
Copy-Item $library (Join-Path $destination 'pax_windows_wgpu_export_probe.dll') -Force
Write-Output (Join-Path $destination 'pax_windows_wgpu_export_probe.dll')
