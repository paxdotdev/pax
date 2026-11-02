param(
    [ValidateSet('ARM64', 'x64')][string]$Architecture = 'x64',
    [string]$Compiler = 'cl'
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
# SDK-only fixture diagnostic; run from the matching MSVC developer shell.
# No WinUI, NuGet, Rust, or input injection. The owner must supervise execution
# with an external 20-second process deadline, even with optional UIA timeouts.
if ($env:VSCMD_ARG_TGT_ARCH -and $env:VSCMD_ARG_TGT_ARCH -ine $Architecture) {
    throw "Developer shell targets $env:VSCMD_ARG_TGT_ARCH, requested $Architecture."
}
$destination = Join-Path $PSScriptRoot "out\$Architecture\uia-selection"
New-Item -ItemType Directory -Force $destination | Out-Null
$executable = Join-Path $destination 'selection-uia.exe'
& $Compiler /nologo /EHsc /W4 /std:c++17 /O1 /MT `
    (Join-Path $PSScriptRoot 'selection-uia.cpp') "/Fo$destination\selection-uia.obj" `
    "/Fe$executable" /link ole32.lib oleaut32.lib user32.lib uiautomationcore.lib
if ($LASTEXITCODE -ne 0) { throw "Native selection diagnostic build failed ($LASTEXITCODE)." }
Write-Output $executable
