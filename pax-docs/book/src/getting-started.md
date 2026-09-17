# Getting Started
<!-- summary: Install Pax, create a project, and run it in your browser. -->
<!-- tags: foundations, cli, build -->

Pax is a UI framework for Rust, with targets for web, macOS, iOS, and iPadOS.
This guide starts with the web target: install Pax, run a project in your
browser, and get to know the files you'll work with.

## Quick start

The CLI's telemetry policy and opt-out controls are described in
[CLI Telemetry](cli-telemetry.md).

If you have Rust, your operating system's build tools, the
`wasm32-unknown-unknown` target, and `wasm-pack` installed, run these commands in
your terminal:

```sh
cargo install pax-cli
pax-cli create my-first-project
cd my-first-project
pax-cli run --target=web
```

Open the local URL printed by the last command. Keep the terminal running while
you use the app; press **Ctrl-C** there to stop it. Installation and the first
build compile dependencies and can take some time.

Need the prerequisites? Start with [Prepare your workstation](#prepare-your-workstation).
Already running? Continue to [Project anatomy](#project-anatomy).

<a id="development-environment-setup"></a>

## Prepare your workstation

You can develop Pax web applications on macOS, Debian/Ubuntu Linux, or Windows.
This guide uses the web target. Pax also runs on macOS, iOS, and iPadOS; building
for those Apple targets requires a macOS workstation with Xcode.

Choose the instructions for your workstation below. If Rust is already
installed, you can skip its installation commands. You'll still need the
WebAssembly target and `wasm-pack` for web builds.

### macOS

Install Xcode Command Line Tools, then complete the installer before continuing:

```sh
xcode-select --install
```

If the tools are already installed, you can continue. Install Rust and the web
build tools:

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
. "$HOME/.cargo/env"

rustup target add wasm32-unknown-unknown
cargo install wasm-pack --version 0.15.0
```

Continue to [Install the Pax CLI](#install-the-pax-cli).

### Linux (Debian / Ubuntu)

Install the system packages, Rust, and the web build tools:

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
```

Continue to [Install the Pax CLI](#install-the-pax-cli).

### Windows

Install Visual Studio Build Tools with the C++ workload, Git, Rust, and the web
build tools from PowerShell. The setup below also installs Clang for dependencies
that need it on ARM64 Windows. It uses `winget` to install Git and may prompt
for administrator approval.

<details>
<summary>Windows setup commands</summary>

Run these commands in order in the same PowerShell session:

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

</details>

Continue in the same PowerShell session to [Install the Pax CLI](#install-the-pax-cli).

## Install the Pax CLI

Cargo is Rust's package manager. Use it to install the published Pax CLI:

```sh
cargo install pax-cli
pax-cli --version
```

The version command confirms that your shell can find the installed CLI.
Installing it compiles Pax and its dependencies, so expect compilation output
before the command finishes.

If you completed the Quick Start, you can skip this installation and the next
two steps.

## Create a project

From the directory where you keep your projects, run:

```sh
pax-cli create my-first-project
```

Choose a destination that does not already exist. The CLI creates Living Quilt,
an interactive geometric tapestry, including its Rust package, UI sources, and
assets. You can [try it in the introduction](what-is-pax.md#try-it-living-quilt).
The project source is bundled with your CLI version; no repository checkout is
needed.

For a smaller starting point, use `pax-cli create my-counter --example=increment`.
The curated alternatives and their purpose are covered in
[Developer Workflow](developer-workflow.md#create-from-a-bundled-example).

## Run it on the web

Enter the project directory and start the web target:

```sh
cd my-first-project
pax-cli run --target=web
```

The first run builds your application and its dependencies. Once the server is
ready, the terminal prints a local URL beginning with `http://127.0.0.1:`. Open
that URL in your browser. Use the address from your current run, since the port
can change between sessions.

Your first-run checkpoint is a geometric quilt with a Pax card in the middle.
Click or tap the quilt to send out a color wave, or click the card to replay
its entrance. The color reveal uses GPU alpha masking; check the
[rendering backend](how-pax-runs.md#rendering-backends) if that effect is absent.
A blank page or a build error means there's still something to resolve;
see [When something goes wrong](#when-something-goes-wrong).

Leave the command running while you use the app. To stop the development session,
press **Ctrl-C** in its terminal. You can start it again with
`pax-cli run --target=web` from the project directory.

## Project anatomy

Open the project in your editor. These are the main roles to recognize:

| File or directory | What it contains |
| --- | --- |
| `Cargo.toml` | The Rust package definition, dependencies, and optional Pax project metadata. |
| Rust source (`.rs`) | Component definitions, application state, event handlers, and other application logic. |
| Pax templates (`.pax`) | UI elements, layout, styles, expressions, and bindings to event handlers. Templates can also be embedded in Rust source. |
| `assets/`, when present | Application media such as images and fonts. |
| `.pax/` | Generated build and development files, created by the CLI. Make application changes in the source files above. |

Pax templates describe the interface. Rust holds the state and behavior behind
it. Expressions inside templates connect property values to what you see on
screen. You'll work with these parts together as you build an interface.

## Make a first edit

Open `src/lib.pax` and find the `LogoCard` near the top. Change its width from:

```pax
width={is_compact ? 290px : 382px}
```

to:

```pax
width={is_compact ? 320px : 420px}
```

Save with the development session still running. The card becomes wider through
Pax's default template hot reload. The expression chooses a compact width for
smaller windows and a larger width otherwise; try resizing the browser. Restore
the original values whenever you like. Rust changes normally require restarting
the run, or opting into [logic reload](developer-workflow.md#hot-reloading).

## When something goes wrong

| Symptom | What to check |
| --- | --- |
| `cargo` or `pax-cli` is not found | Confirm the installation finished successfully. Rust installs commands in `$HOME/.cargo/bin` on macOS/Linux or `%USERPROFILE%\.cargo\bin` on Windows. That directory must be on your shell's `PATH`; reopening the terminal after installation usually picks up the change. |
| A build cannot find the WebAssembly target or its standard library | Run `rustup target add wasm32-unknown-unknown`, then retry. |
| `wasm-pack` is not found | Run `cargo install wasm-pack --version 0.15.0`, then check `wasm-pack --version`. |
| A compiler, linker, or system library is missing | Revisit your [workstation setup](#prepare-your-workstation). On Windows, use the PowerShell session where you loaded the MSVC environment. |
| The project destination already exists | Choose a new directory name. Keep any existing project files. |
| The browser cannot connect | Check that `pax-cli run` is still running and use the local URL from that session. |

For more build detail, run `pax-cli run --target=web --verbose`. If compilation
fails, start with the first reported error. When asking for help, include that
error, your workstation OS, and the output of `pax-cli --version`.

## Where to go next

Continue with [Template Language](template-language.md) to learn how to describe
an interface. [Data Binding and Expressions](data-binding-expressions.md) explains
how values flow into that interface, and [Event Handling with Rust](event-handling-rust.md)
covers responding to input.

For hot reload, inspection, screenshots, and local reference, continue with
[Developer Workflow and Tools](developer-workflow.md).

To choose another target or ship a release, see
[Targets, Build, and Deployment](targets-build-deploy.md).

## Further reference

The links below preserve earlier reference locations. Detailed workflow and
packaging guidance lives in the linked chapters.

### CLI telemetry

The first public CLI command prints the telemetry privacy notice and sends no
telemetry. Later public commands enable minimal telemetry by default. See
[CLI Telemetry](cli-telemetry.md) for the fields and opt-out controls.

### Formatting Pax source

Use `pax-cli fmt` for Pax source and `pax-cli fmt --check` to check without
writing. See [Format Pax source](developer-workflow.md#format-pax-source) for
file selection, inline templates, and CI usage.

### Hot reloading

Debug runs reload `.pax` templates by default. For Rust changes, stop and rerun
the app, or opt into logic reload on web/macOS. See
[Hot reloading](developer-workflow.md#hot-reloading) for the mode matrix,
target boundaries, and project configuration.

### Web public files

Use a project-root `public/` directory for web-only files such as `robots.txt`
and well-known metadata. See [Web public files](targets-build-deploy.md#web-public-files)
for copying, live serving, reserved paths, and the distinction from app assets.

### Project Metadata

Configure titles, icons, Apple identity, and packaging values in
`[package.metadata.pax]` in `Cargo.toml`. See
[Project metadata](targets-build-deploy.md#project-metadata) for the supported
keys, target inheritance, and icon requirements.

### Local release runs on iOS and iPadOS

Use `pax-cli run --release --target ios` or `--target ipados` for an optimized
simulator run. Connected-device runs also accept a device selector and Apple
development team. See [Local release runs](targets-build-deploy.md#local-release-runs)
for signing, device selection, and the boundary with App Store distribution.
