# Getting Started
<!-- summary: How Pax projects are structured and run from the CLI. -->
<!-- tags: foundations, cli, build -->

- Development environment: platform-specific setup guide, dependencies, etc.
- CLI basics: `pax-cli create`, `pax-cli run`, and `pax-cli build` workflows.
- Project layout: `src/lib.pax`, `src/lib.rs`, assets, and Cargo manifest basics.
- Target platforms and build modes: web, macOS, iOS, and iPadOS; debug vs release.
- Assets and URLs: image sources, web fonts, and static files.
- Deployment and packaging overview: where build outputs land and how to ship them.

## Development environment setup

Pax projects are Rust projects. Install the workstation toolchain for your
operating system, then create and run a small project to confirm the setup.

### macOS

#### 1. Install toolchains

Run the following terminal commands to install the dependencies:

```sh
# Install Rust.
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
. "$HOME/.cargo/env"

# Install Xcode Command Line Tools.
xcode-select --install

# Install the WebAssembly target and helper.
rustup target add wasm32-unknown-unknown
cargo install wasm-pack --version 0.15.0

# Install the Pax CLI.
cargo install pax-cli
```

#### 2. Run

Create a new project and run it:

```sh
pax-cli create my-first-project && cd my-first-project && pax-cli run
```

### Linux (Debian / Ubuntu)

#### 1. Install toolchains

These commands install Rust plus the native packages required by Pax web builds.
This dependency set has been validated on Ubuntu 26.04 LTS ARM64.

```sh
# Install native build dependencies.
sudo apt update
sudo apt install -y \
  ca-certificates curl git build-essential pkg-config libssl-dev \
  python3 unzip xvfb \
  libglib2.0-dev libcairo2-dev libpango1.0-dev

# Install Rust.
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
. "$HOME/.cargo/env"

# Install the WebAssembly target and helper.
rustup target add wasm32-unknown-unknown
cargo install wasm-pack --version 0.15.0

# Install the Pax CLI.
cargo install pax-cli
```

#### 2. Run

Create a new project and run it:

```sh
pax-cli create my-first-project && cd my-first-project && pax-cli run
```

### Windows

#### 1. Install toolchains

Install Visual Studio Build Tools with the C++ workload, Git, Rust, and the web
build helper from PowerShell:

```powershell
# Install Visual Studio Build Tools.
$installer = "$env:TEMP\vs_BuildTools.exe"
Invoke-WebRequest https://aka.ms/vs/17/release/vs_BuildTools.exe -OutFile $installer
$vsArgs = @(
  "--quiet",
  "--wait",
  "--norestart",
  "--installPath", "C:\BuildTools",
  "--add", "Microsoft.VisualStudio.Workload.VCTools",
  "--add", "Microsoft.VisualStudio.Component.VC.Llvm.Clang",
  "--add", "Microsoft.VisualStudio.Component.VC.Llvm.ClangToolset",
  "--includeRecommended"
)

if ($env:PROCESSOR_ARCHITECTURE -eq "ARM64") {
  $vsArgs += @("--add", "Microsoft.VisualStudio.Component.VC.Tools.ARM64")
}

Start-Process $installer -Wait -ArgumentList $vsArgs

# Load the MSVC environment in this shell.
$vcvars = "C:\BuildTools\VC\Auxiliary\Build\vcvarsall.bat"
$vcArch = if ($env:PROCESSOR_ARCHITECTURE -eq "ARM64") { "arm64" } else { "x64" }
cmd.exe /s /c "`"$vcvars`" $vcArch >nul && set" | ForEach-Object {
  $separator = $_.IndexOf("=")
  if ($separator -gt 0) {
    Set-Item -Path "Env:$($_.Substring(0, $separator))" `
      -Value $_.Substring($separator + 1)
  }
}

# Make Git and clang available to future shells.
winget install --id Git.Git --exact --source winget `
  --accept-package-agreements --accept-source-agreements

$llvmPath = "C:\BuildTools\VC\Tools\Llvm\bin"
$userPath = @(
  [Environment]::GetEnvironmentVariable("Path", "User") -split ";" |
    Where-Object { $_ }
)
foreach ($path in @("C:\Program Files\Git\cmd", $llvmPath)) {
  if ($userPath -notcontains $path) {
    $userPath = @($userPath) + $path
  }
}
[Environment]::SetEnvironmentVariable("Path", ($userPath -join ";"), "User")
$env:Path = "C:\Program Files\Git\cmd;$llvmPath;$env:Path"

# Install Rust.
$rustupArch = if ($env:PROCESSOR_ARCHITECTURE -eq "ARM64") {
  "aarch64-pc-windows-msvc"
} else {
  "x86_64-pc-windows-msvc"
}

$rustup = "$env:TEMP\rustup-init.exe"
Invoke-WebRequest "https://static.rust-lang.org/rustup/dist/$rustupArch/rustup-init.exe" -OutFile $rustup
& $rustup -y --profile default --default-toolchain stable
$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"

# Install the WebAssembly target and helper.
rustup target add wasm32-unknown-unknown
cargo install wasm-pack --version 0.15.0
```

#### 2. Install pax-cli

```powershell
cargo install pax-cli
```

NOTE: `cargo install pax-cli` and the first `pax-cli run` can take some time.
Subsequent builds are faster.

#### 3. Run

Create and run a smoke project:

```powershell
pax-cli create my-first-project ; cd my-first-project ; pax-cli run
```

## Project Metadata

Rust-backed Pax projects can define build-time project metadata in `Cargo.toml`
under `[package.metadata.pax]`. Pax reads this table while materializing the web
or Apple interface. Cargo ignores the table, so it is safe to keep Pax-specific
packaging data here.

```toml
[package.metadata.pax]
title = "Pax Example"
icon = "assets/icon.png"

[package.metadata.pax.web]
title = "Pax Web"
favicon = "assets/favicon.png"

[package.metadata.pax.ios]
title = "Pax iOS"
bundle_identifier = "dev.pax.example"
development_team = "ABCDE12345"

[package.metadata.pax.ios.info_plist]
NSCameraUsageDescription = "Capture photos when the camera picker is used."

[package.metadata.pax.macos]
title = "Pax macOS"
bundle_identifier = "dev.pax.example.macos"
```

Common keys are inherited by target-specific tables. iPadOS first checks
`[package.metadata.pax.ipados]`, then falls back to iOS, then to common values.
Paths are relative to the project root unless absolute. Explicit CLI flags still
take precedence over Cargo metadata where both exist.

Supported common keys:

- `title`: browser title, Apple display name, and target title default.
- `icon`: shared source image for generated target icons and fallback web favicon.
- `bundle_identifier`: Apple bundle identifier default.
- `marketing_version`: Apple marketing version default. If omitted, Cargo
  `package.version` is used for Apple builds.
- `build_number`: Apple build number.
- `development_team`: Apple development team for signing.
- `info_plist`: shared string values emitted into generated Apple Info.plists.

Supported web keys:

- `title`: overrides the common title for web.
- `favicon`: source file copied as the web favicon.
- `icon`: web-specific icon source used to generate a fallback favicon.

Supported Apple target keys for `ios`, `ipados`, and `macos`:

- `title`
- `icon`
- `bundle_identifier`
- `marketing_version`
- `build_number`
- `development_team`
- `info_plist`: string values emitted into generated Apple Info.plists. Declare
  as a nested table, for example
  `[package.metadata.pax.ios.info_plist] NSCameraUsageDescription = "..."`

iOS and iPadOS use a single generated 1024x1024 `AppIcon` asset. The source
image must be square and opaque. macOS generates the full AppIcon size set from
the same square source image.
