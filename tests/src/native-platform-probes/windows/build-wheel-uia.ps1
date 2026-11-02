param(
    [ValidateSet('ARM64', 'x64')][string]$Architecture = 'x64',
    [string]$Compiler = 'cl'
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
# Read-only SDK fixture observer. Build in the matching MSVC developer shell.
# Every invocation needs an external process deadline, even with UIA timeouts.
if ($env:VSCMD_ARG_TGT_ARCH -and $env:VSCMD_ARG_TGT_ARCH -ine $Architecture) {
    throw "Developer shell targets $env:VSCMD_ARG_TGT_ARCH, requested $Architecture."
}
$destination = Join-Path $PSScriptRoot "out\$Architecture\uia-wheel"
New-Item -ItemType Directory -Force $destination | Out-Null
$executable = Join-Path $destination 'wheel-uia.exe'
& $Compiler /nologo /EHsc /W4 /std:c++17 /O1 /MT `
    (Join-Path $PSScriptRoot 'wheel-uia.cpp') "/Fo$destination\wheel-uia.obj" `
    "/Fe$executable" /link ole32.lib oleaut32.lib user32.lib uiautomationcore.lib
if ($LASTEXITCODE -ne 0) { throw "Native wheel observer build failed ($LASTEXITCODE)." }
Write-Output $executable
