param(
    [ValidateSet('ARM64','x64')][string]$Architecture='x64',
    [string]$Compiler='cl'
)
$ErrorActionPreference='Stop'
Set-StrictMode -Version Latest
if($env:VSCMD_ARG_TGT_ARCH -and $env:VSCMD_ARG_TGT_ARCH -ine $Architecture){throw 'Compiler architecture does not match the requested test'}
$destination=Join-Path $PSScriptRoot "out\$Architecture\keyboard"
New-Item -ItemType Directory -Force $destination|Out-Null
$executable=Join-Path $destination 'keyboard-tests.exe'
& $Compiler /nologo /EHsc /W4 /std:c++17 /O1 /MT (Join-Path $PSScriptRoot 'keyboard-tests.cpp') "/Fo$destination\keyboard-tests.obj" "/Fe$executable" /link user32.lib
if($LASTEXITCODE-ne 0){throw "Keyboard test build failed ($LASTEXITCODE)"}
& $executable
if($LASTEXITCODE-ne 0){throw "Keyboard tests failed ($LASTEXITCODE)"}
