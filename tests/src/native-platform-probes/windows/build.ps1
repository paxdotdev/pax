param(
    [ValidateSet('ARM64', 'x64')][string]$Architecture = 'ARM64',
    [ValidateSet('Debug', 'Release')][string]$Configuration = 'Debug',
    [string]$MSBuild = 'C:\BuildTools\MSBuild\Current\Bin\MSBuild.exe'
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
if (-not (Test-Path $MSBuild)) { throw "MSBuild not found: $MSBuild" }
Set-Location $PSScriptRoot
# Keep this bounded probe's dependencies and generated outputs within its scratch directory.
$env:NUGET_PACKAGES = Join-Path $PSScriptRoot 'packages'
$nuget = Join-Path $PSScriptRoot 'tools\nuget.exe'
if (-not (Test-Path $nuget)) {
    New-Item -ItemType Directory -Force (Split-Path $nuget) | Out-Null
    Invoke-WebRequest 'https://dist.nuget.org/win-x86-commandline/v6.14.0/nuget.exe' -OutFile $nuget
}
& $nuget restore (Join-Path $PSScriptRoot 'packages.config') -PackagesDirectory (Join-Path $PSScriptRoot 'native-packages') `
    -Source 'https://api.nuget.org/v3/index.json' -NonInteractive -Verbosity quiet
if ($LASTEXITCODE -ne 0) { throw "Composition probe dependency restore failed ($LASTEXITCODE)." }
& $MSBuild (Join-Path $PSScriptRoot 'CompositionProbe.vcxproj') /m:2 /v:minimal `
    "/p:Platform=$Architecture" "/p:Configuration=$Configuration"
if ($LASTEXITCODE -ne 0) { throw "Composition probe build failed ($LASTEXITCODE)." }
Write-Output (Join-Path $PSScriptRoot "out\$Architecture\$Configuration\PaxCompositionProbe.exe")
