param(
    [ValidateSet('ARM64', 'x64')][string]$Architecture = 'x64',
    [string]$Compiler = 'cl'
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
# Optional SDK-only probe. Run in the matching MSVC developer shell; no WinUI,
# NuGet restore, Rust build, or changes to the interactive probe are needed.
if ($env:VSCMD_ARG_TGT_ARCH -and $env:VSCMD_ARG_TGT_ARCH -ine $Architecture) {
    throw "Developer shell targets $env:VSCMD_ARG_TGT_ARCH, requested $Architecture."
}
$destination = Join-Path $PSScriptRoot "out\$Architecture\interop"
New-Item -ItemType Directory -Force $destination | Out-Null
$executable = Join-Path $destination 'pax-interop-capabilities.exe'
& $Compiler /nologo /EHsc /std:c++17 /W4 /O1 /MT /DWIN32_LEAN_AND_MEAN /DNOMINMAX `
    (Join-Path $PSScriptRoot 'interop-capabilities.cpp') "/Fo$destination\interop-capabilities.obj" `
    "/Fe$executable" /link d3d12.lib dxgi.lib ole32.lib
if ($LASTEXITCODE -ne 0) { throw "Interop capability probe build failed ($LASTEXITCODE)." }
Write-Output $executable
