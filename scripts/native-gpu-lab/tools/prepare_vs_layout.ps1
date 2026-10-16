# Run on an existing Windows workstation. Downloads a layout without installing VS.
param(
    [Parameter(Mandatory)][string]$BootstrapperUri,
    [Parameter(Mandatory)][string]$BootstrapperSha256,
    [Parameter(Mandatory)][string]$Version,
    [Parameter(Mandatory)][string]$OutputDirectory,
    [Parameter(Mandatory)][string]$ConfigPath
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
if (Test-Path $OutputDirectory) { throw 'Use a new output directory; existing layouts are never overwritten' }
if ($BootstrapperUri -notmatch '^https://download\.visualstudio\.microsoft\.com/' -or $BootstrapperSha256 -cnotmatch '^[0-9a-f]{64}$') { throw 'An official fixed-version bootstrapper and SHA256 are required' }
New-Item -ItemType Directory $OutputDirectory | Out-Null
$bootstrapper = Join-Path $OutputDirectory 'vs_BuildTools.exe'
Invoke-WebRequest -UseBasicParsing $BootstrapperUri -OutFile $bootstrapper
if ((Get-FileHash $bootstrapper -Algorithm SHA256).Hash.ToLowerInvariant() -ne $BootstrapperSha256) { throw 'Bootstrapper hash mismatch' }
if ((Get-AuthenticodeSignature $bootstrapper).Status -ne 'Valid') { throw 'Invalid Microsoft bootstrapper signature' }
$layout = Join-Path $OutputDirectory 'layout'
# The documented default layout architecture is x64, including on an ARM host.
$p = Start-Process $bootstrapper -ArgumentList "--layout `"$layout`" --config `"$ConfigPath`" --lang en-US --quiet --wait" -Wait -PassThru
if ($p.ExitCode -ne 0) { throw "Layout download failed: $($p.ExitCode)" }
$catalog = Get-Content "$layout\Catalog.json" -Raw | ConvertFrom-Json
if ($catalog.info.productDisplayVersion -ne $Version) { throw 'Layout catalog version differs from the selected release' }
$p = Start-Process "$layout\vs_BuildTools.exe" -ArgumentList "--layout `"$layout`" --verify --quiet --wait" -Wait -PassThru
if ($p.ExitCode -ne 0) { throw "Layout verification failed: $($p.ExitCode)" }
Add-Type -AssemblyName System.IO.Compression.FileSystem
$archive = Join-Path $OutputDirectory 'vs-layout.zip'
[IO.Compression.ZipFile]::CreateFromDirectory($layout, $archive, [IO.Compression.CompressionLevel]::Fastest, $false)
@{
    version = $Version
    sha256 = (Get-FileHash $archive -Algorithm SHA256).Hash.ToLowerInvariant()
    bytes = (Get-Item $archive).Length
    bootstrapper_sha256 = $BootstrapperSha256
    catalog_sha256 = (Get-FileHash "$layout\Catalog.json" -Algorithm SHA256).Hash.ToLowerInvariant()
    config_sha256 = (Get-FileHash $ConfigPath -Algorithm SHA256).Hash.ToLowerInvariant()
} | ConvertTo-Json | Set-Content "$OutputDirectory\layout-pin.json"
