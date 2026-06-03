$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"

$Root = "C:\pax-first-touch"
$DoneMarker = Join-Path $Root "workstation-prereqs.done"
$FailedMarker = Join-Path $Root "workstation-prereqs.failed"
$LogPath = Join-Path $Root "workstation-prereqs.log"
$WasmPackVersion = if ($env:PAX_FIRST_TOUCH_WASM_PACK_VERSION) { $env:PAX_FIRST_TOUCH_WASM_PACK_VERSION } else { "0.15.0" }
$RustupToolchain = if ($env:PAX_FIRST_TOUCH_RUSTUP_TOOLCHAIN) { $env:PAX_FIRST_TOUCH_RUSTUP_TOOLCHAIN } else { "stable" }

New-Item -ItemType Directory -Force -Path $Root | Out-Null
if (Test-Path $DoneMarker) {
    exit 0
}
Remove-Item -Force -ErrorAction SilentlyContinue $FailedMarker
Start-Transcript -Path $LogPath -Append | Out-Null

function Add-UserPath {
    param([Parameter(Mandatory = $true)][string]$Path)

    $current = [Environment]::GetEnvironmentVariable("Path", "User")
    $parts = @()
    if ($current) {
        $parts = @($current -split ";" | Where-Object { $_ })
    }
    if ($parts -notcontains $Path) {
        [Environment]::SetEnvironmentVariable("Path", ((@($parts) + $Path) -join ";"), "User")
    }
    $processParts = @($env:Path -split ";" | Where-Object { $_ })
    if ($processParts -notcontains $Path) {
        $env:Path = ((@($Path) + $processParts) -join ";")
    }
}

function Invoke-Installer {
    param(
        [Parameter(Mandatory = $true)][string]$FilePath,
        [Parameter(Mandatory = $true)][string[]]$ArgumentList,
        [int[]]$AllowedExitCodes = @(0)
    )

    $process = Start-Process -FilePath $FilePath -ArgumentList $ArgumentList -Wait -PassThru
    if ($AllowedExitCodes -notcontains $process.ExitCode) {
        throw "$FilePath exited with $($process.ExitCode)"
    }
}

function Invoke-NativeCommand {
    param(
        [Parameter(Mandatory = $true)][string]$FilePath,
        [string[]]$ArgumentList = @(),
        [int[]]$AllowedExitCodes = @(0)
    )

    & $FilePath @ArgumentList
    if ($AllowedExitCodes -notcontains $LASTEXITCODE) {
        throw "$FilePath exited with $LASTEXITCODE"
    }
}

function Import-MSVCEnvironment {
    $vcvars = "C:\BuildTools\VC\Auxiliary\Build\vcvarsall.bat"
    if (-not (Test-Path $vcvars)) {
        throw "Visual Studio Build Tools vcvarsall.bat was not found at $vcvars"
    }

    $arch = if ($env:PROCESSOR_ARCHITECTURE -eq "ARM64") { "arm64" } else { "x64" }
    $environment = cmd.exe /s /c "`"$vcvars`" $arch >nul && set"
    foreach ($line in $environment) {
        $separator = $line.IndexOf("=")
        if ($separator -gt 0) {
            $name = $line.Substring(0, $separator)
            $value = $line.Substring($separator + 1)
            Set-Item -Path "Env:$name" -Value $value
        }
    }
}

function Test-MSVCLinker {
    $hostArch = if ($env:PROCESSOR_ARCHITECTURE -eq "ARM64") { "Hostarm64" } else { "Hostx64" }
    $targetArch = if ($env:PROCESSOR_ARCHITECTURE -eq "ARM64") { "arm64" } else { "x64" }
    $linkerPattern = "\\bin\\$hostArch\\$targetArch\\link.exe$"
    $linkers = Get-ChildItem "C:\BuildTools\VC\Tools\MSVC" -Recurse -Filter link.exe -ErrorAction SilentlyContinue
    return ($linkers | Where-Object { $_.FullName -match $linkerPattern } | Select-Object -First 1) -ne $null
}

function Test-Clang {
    if (Get-Command clang.exe -ErrorAction SilentlyContinue) {
        return $true
    }
    return (Test-Path "C:\BuildTools\VC\Tools\Llvm\bin\clang.exe")
}

try {
    [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12

    if (-not (Test-Path "C:\BuildTools\VC\Auxiliary\Build\vcvarsall.bat") -or -not (Test-MSVCLinker) -or -not (Test-Clang)) {
        $vsInstaller = Join-Path $env:TEMP "vs_BuildTools.exe"
        Invoke-WebRequest -Uri "https://aka.ms/vs/17/release/vs_BuildTools.exe" -OutFile $vsInstaller
        $vsArgs = @(
            "--quiet",
            "--wait",
            "--norestart",
            "--nocache",
            "--installPath", "C:\BuildTools",
            "--add", "Microsoft.VisualStudio.Workload.VCTools",
            "--add", "Microsoft.VisualStudio.Component.VC.Llvm.Clang",
            "--add", "Microsoft.VisualStudio.Component.VC.Llvm.ClangToolset",
            "--includeRecommended"
        )
        if ($env:PROCESSOR_ARCHITECTURE -eq "ARM64") {
            $vsArgs += @("--add", "Microsoft.VisualStudio.Component.VC.Tools.ARM64")
        }
        Invoke-Installer -FilePath $vsInstaller -AllowedExitCodes @(0, 3010) -ArgumentList $vsArgs
    }
    Import-MSVCEnvironment
    if (Test-Path "C:\BuildTools\VC\Tools\Llvm\bin") {
        Add-UserPath "C:\BuildTools\VC\Tools\Llvm\bin"
    }

    if (-not (Get-Command git.exe -ErrorAction SilentlyContinue)) {
        if (-not (Get-Command winget.exe -ErrorAction SilentlyContinue)) {
            throw "winget.exe is required to install Git for Windows"
        }
        Invoke-NativeCommand -FilePath "winget.exe" -ArgumentList @(
            "install",
            "--id", "Git.Git",
            "--exact",
            "--source", "winget",
            "--silent",
            "--accept-package-agreements",
            "--accept-source-agreements"
        )
    }
    if (Test-Path "C:\Program Files\Git\cmd") {
        Add-UserPath "C:\Program Files\Git\cmd"
    }

    if (-not (Get-Command node.exe -ErrorAction SilentlyContinue) -or -not (Get-Command npm.cmd -ErrorAction SilentlyContinue)) {
        if (-not (Get-Command winget.exe -ErrorAction SilentlyContinue)) {
            throw "winget.exe is required to install Node.js LTS"
        }
        Invoke-NativeCommand -FilePath "winget.exe" -ArgumentList @(
            "install",
            "--id", "OpenJS.NodeJS.LTS",
            "--exact",
            "--source", "winget",
            "--silent",
            "--accept-package-agreements",
            "--accept-source-agreements"
        )
    }
    if (Test-Path "C:\Program Files\nodejs") {
        Add-UserPath "C:\Program Files\nodejs"
    }

    $cargoBin = Join-Path $env:USERPROFILE ".cargo\bin"
    Add-UserPath $cargoBin

    if (-not (Get-Command rustup.exe -ErrorAction SilentlyContinue)) {
        $rustupArch = if ($env:PROCESSOR_ARCHITECTURE -eq "ARM64") { "aarch64-pc-windows-msvc" } else { "x86_64-pc-windows-msvc" }
        $rustupInit = Join-Path $env:TEMP "rustup-init.exe"
        Invoke-WebRequest -Uri "https://static.rust-lang.org/rustup/dist/$rustupArch/rustup-init.exe" -OutFile $rustupInit
        Invoke-Installer -FilePath $rustupInit -ArgumentList @("-y", "--profile", "default", "--default-toolchain", $RustupToolchain)
    }

    Invoke-NativeCommand -FilePath "rustup.exe" -ArgumentList @("toolchain", "install", $RustupToolchain)
    Invoke-NativeCommand -FilePath "rustup.exe" -ArgumentList @("default", $RustupToolchain)
    Invoke-NativeCommand -FilePath "rustup.exe" -ArgumentList @("target", "add", "wasm32-unknown-unknown")

    $wasmPackReady = $false
    if (Get-Command wasm-pack.exe -ErrorAction SilentlyContinue) {
        $wasmPackReady = ((wasm-pack --version) -eq "wasm-pack $WasmPackVersion")
    }
    if (-not $wasmPackReady) {
        Invoke-NativeCommand -FilePath "cargo.exe" -ArgumentList @("install", "wasm-pack", "--version", $WasmPackVersion)
    }

    $versions = @(
        "pax-first-touch windows system prerequisites ready",
        (rustc --version),
        (cargo --version),
        (rustup --version),
        (node --version),
        (npm --version),
        (wasm-pack --version),
        (git --version)
    )
    $versions | Set-Content -Path $DoneMarker -Encoding UTF8
}
catch {
    $_ | Out-String | Set-Content -Path $FailedMarker -Encoding UTF8
    throw
}
finally {
    Stop-Transcript | Out-Null
}
