param(
    [ValidateSet('ARM64', 'x64')][string]$Architecture = 'x64',
    [ValidateSet('Debug', 'Release')][string]$Configuration = 'Debug',
    [string]$OutputDirectory,
    [string]$MSBuild
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
& $nuget restore (Join-Path $sourceDirectory 'packages.config') -PackagesDirectory (Join-Path $sourceDirectory 'native-packages') `
    -Source 'https://api.nuget.org/v3/index.json' -NonInteractive -Verbosity quiet
if ($LASTEXITCODE -ne 0) { throw "PaxHost dependency restore failed ($LASTEXITCODE)." }
& $MSBuild (Join-Path $sourceDirectory 'PaxHost.vcxproj') /m:2 /v:minimal `
    "/p:Platform=$Architecture" "/p:Configuration=$Configuration" "/p:OutputDirectory=$OutputDirectory"
if ($LASTEXITCODE -ne 0) { throw "PaxHost build failed ($LASTEXITCODE)." }
Write-Output (Join-Path $OutputDirectory 'PaxHost.dll')
