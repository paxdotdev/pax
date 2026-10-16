param(
    [ValidateSet('ARM64', 'x64')][string]$Architecture = 'x64',
    [string]$Compiler = 'cl'
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
# SDK-only, read-only UIA client. Run in the matching MSVC developer shell;
# no WinUI, NuGet restore, Rust build, or input injection is involved.
# CUIAutomation may not expose IUIAutomation2's optional timeouts. Always run
# the resulting diagnostic under an external process deadline.
if ($env:VSCMD_ARG_TGT_ARCH -and $env:VSCMD_ARG_TGT_ARCH -ine $Architecture) {
    throw "Developer shell targets $env:VSCMD_ARG_TGT_ARCH, requested $Architecture."
}
$destination = Join-Path $PSScriptRoot "out\$Architecture\uia"
New-Item -ItemType Directory -Force $destination | Out-Null
$executable = Join-Path $destination 'native-uia.exe'
& $Compiler /nologo /EHsc /W4 /std:c++17 /O1 /MT `
    (Join-Path $PSScriptRoot 'native-uia.cpp') "/Fo$destination\native-uia.obj" `
    "/Fe$executable" /link ole32.lib oleaut32.lib user32.lib uiautomationcore.lib
if ($LASTEXITCODE -ne 0) { throw "Native UI Automation diagnostic build failed ($LASTEXITCODE)." }
Write-Output $executable
