# EC2 guest only. RuntimeOnly stages the probe runtime on a pinned NVIDIA AMI;
# the default builds the full Packer development image.
param([switch]$RuntimeOnly)

# No credentials are accepted from the manifest.
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
# Explicit imports avoid first-use command discovery stalls on the vendor AMI.
Import-Module Microsoft.PowerShell.Management, Microsoft.PowerShell.Utility, Microsoft.PowerShell.Security, CimCmdlets -ErrorAction Stop
if ((Get-CimInstance Win32_ComputerSystem).Manufacturer -ne 'Amazon EC2') { throw 'EC2 guest required' }
$os = Get-CimInstance Win32_OperatingSystem
if ($os.Caption -notmatch 'Windows Server 2022' -or $env:PROCESSOR_ARCHITECTURE -ne 'AMD64') { throw 'Windows Server 2022 x64 required' }
$inputs = Get-Content C:\Windows\Temp\pax-lab-inputs.json -Raw | ConvertFrom-Json
if ($inputs.schema -ne 1 -or $inputs.guest_os -ne 'windows' -or $inputs.rust_version -notmatch '^\d+\.\d+\.\d+$') { throw 'Invalid manifest' }
$ids = @('rustup', 'grid', 'dcv', 'vs_layout', 'git', 'vc_redist', 'winappsdk')
if (Compare-Object $ids @($inputs.artifacts.PSObject.Properties.Name)) { throw 'Unexpected artifact IDs' }
foreach ($id in $ids) {
    $a = $inputs.artifacts.$id
    if ($a.sha256 -cnotmatch '^[0-9a-f]{64}$' -or $a.sha256 -match '^(.)\1{63}$') { throw "Missing verified SHA256: $id" }
    if ($a.version -notmatch '^[0-9][A-Za-z0-9._+-]*$') { throw "Unpinned version: $id" }
    $uri = [Uri]$a.uri
    if ($uri.Scheme -notin @('https', 's3') -or -not $uri.Host -or $uri.AbsolutePath -eq '/' -or $uri.Query -or $uri.Fragment -or $uri.UserInfo -or $a.uri -match 'latest|REPLACE|REVIEW_REQUIRED') { throw "Invalid artifact URI: $id" }
}
$root = 'C:\PaxGpuLab'
$downloads = Join-Path $root 'downloads'
New-Item -ItemType Directory -Force $downloads | Out-Null
Copy-Item C:\Windows\Temp\pax-lab-inputs.json "$root\inputs.json"
Start-Transcript "$root\setup.log" | Out-Null
function Fetch([string]$id, [string]$extension) {
    $a = $inputs.artifacts.$id
    $output = Join-Path $downloads "$id.$extension"
    $uri = [Uri]$a.uri
    if ($uri.Scheme -eq 's3') {
        Read-S3Object -BucketName $uri.Host -Key $uri.AbsolutePath.TrimStart('/') -File $output -Region us-west-2 | Out-Null
    } else {
        Invoke-WebRequest -UseBasicParsing -Uri $uri.AbsoluteUri -OutFile $output
    }
    if ((Get-FileHash $output -Algorithm SHA256).Hash.ToLowerInvariant() -ne $a.sha256) { throw "Hash mismatch: $id" }
    return $output
}
function Install([string]$file, [string]$arguments) {
    # Wait on this installer, not the inherited SSM process job and its descendants.
    $p = Start-Process -FilePath $file -ArgumentList $arguments -PassThru
    try {
        $processHandle = $p.Handle
        $timeout = if ($RuntimeOnly) { 300000 } else { 3600000 }
        if (-not $p.WaitForExit($timeout)) { throw "$file exceeded its installation deadline; inspect the installer process before retrying" }
        $p.Refresh()
        if ($p.ExitCode -notin @(0, 3010)) { throw "$file failed: $($p.ExitCode)" }
    } finally { $p.Dispose() }
}
try {
    [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
    $files = @{}
    $installIds = if ($RuntimeOnly) { @('vc_redist', 'dcv', 'winappsdk') } else { $ids }
    foreach ($id in $installIds) {
        $extension = if ($id -eq 'vs_layout') { 'zip' } elseif ($id -eq 'dcv') { 'msi' } else { 'exe' }
        $files[$id] = Fetch $id $extension
    }
    if (-not $RuntimeOnly) {
        # Keep the complete layout's installer, catalog and channel together.
        $layout = Join-Path $downloads 'vs-layout'
        Expand-Archive -Path $files.vs_layout -DestinationPath $layout
        Install "$layout\vs_BuildTools.exe" "--quiet --wait --norestart --noWeb --noUpdateInstaller --installPath C:\BuildTools --config C:\Windows\Temp\pax-buildtools.vsconfig --channelUri `"$layout\ChannelManifest.json`""
        Install $files.git '/VERYSILENT /NORESTART /SP-'
        Install $files.grid '-s -noreboot'
    }
    Install $files.vc_redist '/install /quiet /norestart'
    Install 'msiexec.exe' "/i `"$($files.dcv)`" /quiet /norestart DISABLE_SERVER_AUTOSTART=1 /l*v `"$root\dcv-install.log`""
    # Retain installer for registration in the eventual interactive user's context.
    Copy-Item $files.winappsdk "$root\WindowsAppRuntimeInstall.exe"

    if (-not $RuntimeOnly) {
        $env:CARGO_HOME = "$root\cargo"
        $env:RUSTUP_HOME = "$root\rustup"
        [Environment]::SetEnvironmentVariable('CARGO_HOME', $env:CARGO_HOME, 'Machine')
        [Environment]::SetEnvironmentVariable('RUSTUP_HOME', $env:RUSTUP_HOME, 'Machine')
        Install $files.rustup "-y --profile minimal --no-modify-path --default-toolchain $($inputs.rust_version)"
        $machinePath = [Environment]::GetEnvironmentVariable('Path', 'Machine')
        [Environment]::SetEnvironmentVariable('Path', "$root\cargo\bin;C:\Program Files\Git\cmd;$machinePath", 'Machine')
        & "$root\cargo\bin\rustc.exe" --version | Set-Content "$root\rust.txt"
        if ($LASTEXITCODE -ne 0) { throw 'Rust installation failed' }
        if (-not (Test-Path C:\BuildTools\VC\Auxiliary\Build\vcvarsall.bat)) { throw 'MSVC missing' }
        Move-Item C:\Windows\Temp\pax-buildtools.vsconfig "$root\buildtools.vsconfig"
    }
    Get-CimInstance Win32_OperatingSystem | Select-Object Caption,Version,BuildNumber | ConvertTo-Json | Set-Content "$root\os.json"
    Get-CimInstance Win32_VideoController | Select-Object Name,DriverVersion,PNPDeviceID | ConvertTo-Json | Set-Content "$root\video-controllers.json"
    # Freeze the short-lived test image. Refresh through a reviewed rebuild.
    $updatePolicy = 'HKLM:\SOFTWARE\Policies\Microsoft\Windows\WindowsUpdate\AU'
    New-Item $updatePolicy -Force | Out-Null
    New-ItemProperty $updatePolicy -Name NoAutoUpdate -Value 1 -PropertyType DWord -Force | Out-Null
    Set-Service dcvserver -StartupType Automatic
    Remove-Item -Recurse -Force $downloads
    Remove-Item C:\Windows\Temp\pax-lab-inputs.json
} finally {
    Stop-Transcript | Out-Null
}
