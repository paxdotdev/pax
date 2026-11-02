param(
    [ValidateSet('ARM64','x64')][string]$Architecture='x64',
    [string]$Compiler='cl'
)
$ErrorActionPreference='Stop'
Set-StrictMode -Version Latest
# Read-only SDK UIA client. Run in the matching MSVC developer shell and use
# an external process deadline when querying the interactive guest desktop.
if($env:VSCMD_ARG_TGT_ARCH -and $env:VSCMD_ARG_TGT_ARCH -ine $Architecture){throw 'Compiler architecture does not match the requested diagnostic'}
$destination=Join-Path $PSScriptRoot "out\$Architecture\uia"
New-Item -ItemType Directory -Force $destination|Out-Null
$executable=Join-Path $destination 'ime-uia.exe'
& $Compiler /nologo /EHsc /W4 /std:c++17 /O1 /MT (Join-Path $PSScriptRoot 'ime-uia.cpp') "/Fo$destination\ime-uia.obj" "/Fe$executable" /link ole32.lib oleaut32.lib user32.lib uiautomationcore.lib
if($LASTEXITCODE-ne 0){throw "Native IME observation diagnostic build failed ($LASTEXITCODE)"}
Write-Output $executable
