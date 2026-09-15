# Getting Started
<!-- summary: How Pax projects are structured and run from the CLI. -->
<!-- tags: foundations, cli, build -->

- Development environment: platform-specific setup guide, dependencies, etc.
- CLI basics: `pax-cli create`, `pax-cli run`, and `pax-cli build` workflows.
- Hot-reload policy: independently control Pax and application-logic updates.
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

## Bundled starters

`pax-cli create` (also available as `pax-cli new`) starts from **Living Quilt**:
the animated Pax logo on an interactive geometric tapestry. Select a different
bundled example by its string ID:

```sh
pax-cli create my-quilt                           # living-quilt (default)
pax-cli create my-postcard --example ink-and-light
pax-cli create my-counter --example increment
```

These starters are self-contained snapshots shipped with the installed CLI,
not downloads of a moving branch. Newer canonical example changes arrive with
a newer CLI build; existing generated projects are not overwritten.

For Pax contributors, `examples/src/*` is the sole hand-edited source. The
registry in `examples/bundled-cli-examples.toml` maps each bundled ID to its
source directory and selects the default. Run `scripts/sync-cli-examples.py`
after changing a curated example; `--check` verifies that the tracked,
deterministic `pax-compiler/files/new-project/bundled-examples.paxbundle` is
current. CI checks for drift, and release preparation syncs after version
rewriting. The compiler embeds that crate-owned artifact, so installed crates
never need files from outside their package. Do not hand-edit a second starter
implementation in the CLI.

## Debug build defaults

Examples and generated projects optimize runtime dependencies at level 1 while
leaving the application crate at level 0. This keeps the runtime responsive
without paying for optimization of the application's Rust code on every edit:

```toml
[profile.dev]
opt-level = 1

[profile.dev.package.my-quilt]
opt-level = 0
```

Use the application's exact Cargo package name for the second table. Creation
rewrites it automatically; if you later rename `package.name` yourself, rename
its package profile override too. Cargo reads profiles from the workspace
root, so move these settings there if you add the app to a larger workspace.

Debug information, debug assertions, overflow checks, incremental compilation,
and hot reload retain their normal development behavior. Build scripts and
proc macros retain Cargo's unoptimized defaults. Avoid replacing this with a
`[profile.dev.package."*"]` override: the wildcard also takes precedence over
build-dependency defaults. Release profiles are unchanged.

Optimized dependencies take longer to compile the first time, but are cached
for subsequent application edits. For fully unoptimized debugging, override
the profile default for one invocation:

```sh
CARGO_PROFILE_DEV_OPT_LEVEL=0 pax-cli run
```

In PowerShell, set `$env:CARGO_PROFILE_DEV_OPT_LEVEL = "0"` before running the
command, and remove it afterward with
`Remove-Item Env:CARGO_PROFILE_DEV_OPT_LEVEL` to return to the project default.

## Formatting Pax source

Run `pax-cli fmt` from a project or workspace root to recursively format `.pax`
files and Pax templates embedded in Rust source. Generated and dependency
directories such as `.pax`, `target`, and `node_modules` are skipped.

```sh
pax-cli fmt                 # format the current directory
pax-cli fmt path/to/project # format a file or directory
pax-cli fmt --check         # report drift without writing files
```

Formatted files always end with one newline. `--check` exits unsuccessfully
when any file would change, so the same command can enforce canonical Pax
formatting in CI; it does not inspect whether the process happens to be running
in a CI environment.

## CLI telemetry

The first public CLI command prints the telemetry privacy notice and sends no
telemetry. Later public commands enable minimal telemetry by default. See [CLI
Telemetry](cli-telemetry.md) for the exact fields, server-side coarse-location
handling, and opt-out controls.

## Hot reloading

Debug `pax-cli run` sessions reload Pax UI sources by default. Application
logic reload is opt-in so a Rust edit does not unexpectedly begin a long
background build. The two lanes are independent: a `.pax` edit can update the
mounted tree without replacing application logic, while an enabled logic lane
builds and activates a new compiled artifact. `logic` is intentionally
language-neutral so the same policy can cover Rust today and interpreted
application-logic modules in the future.

Use `--hot-reload` to select the lanes for one run:

```sh
pax-cli run --hot-reload=pax   # .pax only (default)
pax-cli run --hot-reload=all   # .pax and application logic
pax-cli run --hot-reload=logic # application logic only
pax-cli run --hot-reload=off   # neither lane
```

The generated `cargo run` wrapper forwards its trailing arguments, so
`cargo run -- --hot-reload=pax` selects the same policy.

Web and macOS support both lanes. iOS and iPadOS support Pax hot reload, but
application-logic changes require rebuilding and relaunching the app;
`--hot-reload=logic` therefore reports an unsupported-mode error on those
targets. With `all`, mobile sessions continue to reload `.pax` changes and
report once when a saved logic change requires a restart.

Disabling a lane suppresses activation in the running app, not source writes.
Edits remain on disk and enter the next permitted logic build or app restart.
The designtime server also remains available for read-only inspection when hot
reload is `off`.

For a persistent project default, use Cargo metadata:

```toml
[package.metadata.pax.dev]
hot_reload = "pax"
```

For a shell or tool invocation, set `PAX_HOT_RELOAD=all|pax|logic|off`.
Precedence is CLI flag, environment variable, Cargo metadata, then the `pax`
debug default. Release builds always force hot reload `off`; release cartridges
support neither `.pax` live reload nor dynamic or interpreted logic
replacement.

## Local release runs on iOS and iPadOS

Use `--release` to compile, development-sign, install, and launch an optimized
app on a connected device:

```sh
pax-cli run --release --target ipados \
  --ios-device 'device:My iPad' --ios-development-team YOUR_TEAM_ID
```

Use `--target ios` for iPhone. The selector accepts a device name or hardware
UDID; Xcode must have access to the team's development signing credentials and
the device must be paired with Developer Mode enabled. The team can also be
configured in Cargo metadata (below). Without `--ios-device`, `run` selects a
simulator; an explicit `--ios-device simulator:<name-or-udid>` also works in
release mode and does not require signing credentials.

This is a local development workflow, not App Store distribution: the CLI uses
Xcode's Release configuration with Apple Development signing, and does not
archive, export, or upload the app. `build --release --target ios|ipados` builds
without launching. Release apps disable designtime and both hot-reload lanes,
even when requested by flags or project settings. `run --release` is currently
supported only for iOS and iPadOS.

## Web public files

Create a `public/` directory beside `Cargo.toml` when a web application needs
files served directly from the site root. Pax preserves each file's relative
path and bytes:

```text
public/ai.md             -> /ai.md
public/robots.txt        -> /robots.txt
public/.well-known/pax   -> /.well-known/pax
public/guide/index.html  -> /guide/
```

`pax-cli build --target web` copies these files into the deployable web output.
During `pax-cli run`, the development server reads them directly from the
project's `public/` directory. Edits, additions, and deletions are therefore
visible after a browser refresh without rebuilding or restarting Pax.

Public files cannot replace generated web files or runtime-owned directories.
For example, `public/index.html`, `public/assets/`, `public/snippets/`, and
`public/__reloads__/` are rejected. Symbolic links are also rejected so a web
build cannot accidentally publish files outside the project.

Use `assets/` for application media loaded by Pax across targets; those files
are addressed beneath `/assets`. Use `public/` for web-only responses that must
exist before the Pax runtime loads, such as text documents, robots directives,
or well-known metadata. `Router` remains responsible for selecting live
application UI after startup and does not declare static HTTP responses.

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

Supported development keys under `[package.metadata.pax.dev]`:

- `hot_reload`: `all`, `pax`, `logic`, or `off`. See [Hot
  reloading](#hot-reloading) for target support and precedence.

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
the same square source image. The bundled iOS/iPadOS interface includes a Pax
icon when no custom icon is configured; ejected interfaces retain their own
asset catalog unless an icon override is provided.
