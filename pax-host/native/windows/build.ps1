param(
    [ValidateSet('ARM64', 'x64')][string]$Architecture = 'x64',
    [ValidateSet('Debug', 'Release')][string]$Configuration = 'Debug',
    [string]$OutputDirectory,
    [string]$MSBuild,
    [string]$PackagesDirectory
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
function ConvertTo-BuildPath([string]$Path) {
    # Rust canonical paths use extended prefixes which Windows PowerShell 5's
    # filesystem provider cannot join. Preserve UNC roots and other devices.
    if ($Path.StartsWith('\\?\UNC\', [StringComparison]::OrdinalIgnoreCase)) {
        return '\\' + $Path.Substring(8)
    }
    if ($Path -match '^\\\\\?\\[A-Za-z]:\\') { return $Path.Substring(4) }
    return $Path
}
$sourceDirectory = ConvertTo-BuildPath $PSScriptRoot
if (-not $MSBuild) {
    $candidate = Get-Command MSBuild.exe -ErrorAction SilentlyContinue
    if ($candidate) { $MSBuild = $candidate.Source }
    elseif (Test-Path 'C:\BuildTools\MSBuild\Current\Bin\MSBuild.exe') {
        $MSBuild = 'C:\BuildTools\MSBuild\Current\Bin\MSBuild.exe'
    } else {
        $vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
        if (Test-Path $vswhere) {
            $MSBuild = & $vswhere -latest -products '*' -requires Microsoft.Component.MSBuild -find 'MSBuild\**\Bin\MSBuild.exe' | Select-Object -First 1
        }
    }
}
if (-not $MSBuild -or -not (Test-Path $MSBuild)) { throw 'MSBuild with the v143 C++ workload is required.' }
if (-not $OutputDirectory) { $OutputDirectory = Join-Path $sourceDirectory "out\$Architecture\$Configuration" }
$OutputDirectory = ConvertTo-BuildPath $OutputDirectory
$OutputDirectory = [IO.Path]::GetFullPath($OutputDirectory)
New-Item -ItemType Directory -Force $OutputDirectory | Out-Null
$nuget = Join-Path $sourceDirectory 'tools\nuget.exe'
if (-not (Test-Path $nuget)) {
    New-Item -ItemType Directory -Force (Split-Path $nuget) | Out-Null
    Invoke-WebRequest 'https://dist.nuget.org/win-x86-commandline/v6.14.0/nuget.exe' -OutFile $nuget
}
# WinAppSDK contains deeply nested package paths. Restoring beside an authored
# project can exceed NuGet's Windows path limit even for ordinary fixture paths.
if (-not $PackagesDirectory) {
    if ([Security.Principal.WindowsIdentity]::GetCurrent().IsSystem) {
        # SYSTEM's LocalAppData lives under System32, which 32-bit WinAppSDK
        # tools see through WOW64 redirection. Keep service builds outside it.
        $PackagesDirectory = Join-Path ([Environment]::GetFolderPath('CommonApplicationData')) 'Pax\NuGet\SYSTEM'
    } else {
        $PackagesDirectory = Join-Path ([Environment]::GetFolderPath('LocalApplicationData')) 'Pax\NuGet'
    }
}
$PackagesDirectory = [IO.Path]::GetFullPath((ConvertTo-BuildPath $PackagesDirectory))
New-Item -ItemType Directory -Force $PackagesDirectory | Out-Null
& $nuget restore (Join-Path $sourceDirectory 'packages.config') -PackagesDirectory $PackagesDirectory `
    -Source 'https://api.nuget.org/v3/index.json' -NonInteractive -Verbosity quiet
if ($LASTEXITCODE -ne 0) { throw "PaxHost dependency restore failed ($LASTEXITCODE)." }
# Native contract fixtures reuse this exact restored dependency set.
@{ packagesDirectory = $PackagesDirectory } | ConvertTo-Json | Set-Content (Join-Path $sourceDirectory 'host-packages.json') -Encoding UTF8
& $MSBuild (Join-Path $sourceDirectory 'PaxHost.vcxproj') /m:2 /v:minimal `
    "/p:Platform=$Architecture" "/p:Configuration=$Configuration" "/p:OutputDirectory=$OutputDirectory" `
    "/p:HostPackagesRoot=$PackagesDirectory/"
if ($LASTEXITCODE -ne 0) { throw "PaxHost build failed ($LASTEXITCODE)." }
Write-Output (Join-Path $OutputDirectory 'PaxHost.dll')
