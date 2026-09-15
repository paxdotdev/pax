# Pax first-touch workstation harness

This directory holds reproducible local VM setup for validating the public Pax
first-touch flow outside the Pax monorepo. The harness should create a clean
workstation baseline and then run smoke scenarios that use the same commands a
new user would use.

The harness intentionally keeps OS provisioning separate from Pax validation:

- `host/` scripts run on the macOS workstation and create/configure VMs.
- `guest/` files are consumed inside the VM during install or smoke runs.

The source-linked smoke baselines include Node.js/npm because they validate the
monorepo source checkout, where the web interface bundle is an ignored generated
artifact and may need to be rebuilt from TypeScript. Published `pax-cli` users
should receive that bundle from the release package and should not need Node.js
for the normal getting-started path.

## macOS

Build the macOS ARM64 VM from the Pax repo root:

```sh
scripts/first-touch/host/provision-macos.sh
```

Defaults:

- VM name: `pax-macos-first-touch`
- macOS image: latest host-supported Apple Silicon restore IPSW from Apple's
  `com_apple_macOSIPSW` catalog, cached under
  `~/.cache/pax-first-touch/ipsw/`
- VM bundle: `~/Parallels/pax-macos-first-touch.macvm`
- VM resources: 4 CPUs, 8192 MiB RAM
- Login user: `pax`
- Rust toolchain: `stable`
- Node.js: 22.22.3
- WebAssembly helper: `wasm-pack` 0.15.0

Set `PAX_FIRST_TOUCH_MACOS_IPSW` to use a pinned local restore image instead of
resolving and downloading from Apple's catalog. Set `PAX_FIRST_TOUCH_PASSWORD`
to control the VM password. If omitted, the script generates one for the current
invocation and prints it once.

macOS guests do not currently have an unattended account-creation answer file in
this harness. If Parallels stops at Setup Assistant, open the console, create
the configured local admin user, and rerun the provisioning script with
`PAX_FIRST_TOUCH_REUSE_EXISTING=1` and the same password. Once Parallels guest
control can authenticate, the script installs Command Line Tools, Rust, the
WebAssembly target, Node.js, and `wasm-pack`, then creates the
`pax-first-touch-macos-workstation-prereqs` snapshot.

To run the source-linked smoke against a running or stopped VM:

```sh
PAX_FIRST_TOUCH_PASSWORD=... scripts/first-touch/host/smoke-macos-source.sh
```

The smoke copies a tarball of this checkout into the guest over `prlctl exec`
stdin, installs `pax-cli` from source, creates a fresh project outside the
monorepo, patches `pax-kit` back to the synced source tree, builds the
generated project for web, and launches `pax-cli run --target web` long enough
to verify that the local web server responds.

## Ubuntu

Build the Ubuntu ARM64 VM from the Pax repo root:

```sh
scripts/first-touch/host/provision-ubuntu.sh
```

Defaults:

- VM name: `pax-ubuntu-first-touch`
- Ubuntu image: 26.04 LTS ARM64 live server ISO from Canonical
- VM resources: 4 CPUs, 6144 MiB RAM, 65536 MiB sparse disk
- Login user: `pax`
- Rust toolchain: `stable`
- Node.js and npm from Ubuntu packages
- WebAssembly helper: `wasm-pack` 0.15.0

Set `PAX_FIRST_TOUCH_PASSWORD` to control the temporary VM password. If omitted,
the script generates one for the current invocation, stores it in a mode-0600
file under `~/.cache/pax-first-touch/keys/`, and passes it to Packer via
environment variables. Override the password-file location with
`PAX_FIRST_TOUCH_PASSWORD_FILE`. The
provisioner installs
`~/.cache/pax-first-touch/keys/pax-ubuntu-first-touch_ed25519.pub` for the `pax`
user by default, generating the host-local keypair there when it is absent.
Override the pair with `PAX_FIRST_TOUCH_SSH_KEY` or provide a public key with
`PAX_FIRST_TOUCH_SSH_PUBLIC_KEY`. The matching private key remains host-local
and is used by the source smoke.

The first successful output should be a stopped and registered Parallels VM with
a `pax-first-touch-ubuntu-workstation-prereqs` snapshot. Pax smoke scripts
should run against that baseline or a clone/snapshot of it.

The baseline intentionally includes the Linux development packages needed by
Pax's current source-linked web build graph: Node.js/npm, GLib, Cairo, and Pango
headers. Without these, source-built `pax-cli` can fail before a user reaches a
generated app.

To run the source-linked smoke against a running VM, set up key or agent-based
SSH access for the `pax` user and run:

```sh
scripts/first-touch/host/smoke-ubuntu-source.sh
```

The smoke syncs this checkout into the guest, installs `pax-cli` from source,
creates a fresh project outside the monorepo, patches `pax-kit` back to the
synced source tree, and builds the generated project for web.

## Windows

Build the Windows 11 ARM64 VM from the Pax repo root:

```sh
scripts/first-touch/host/provision-windows.sh
```

Defaults:

- VM name: `pax-windows-first-touch`
- Windows image: `~/.cache/pax-first-touch/iso/Win11_25H2_English_Arm64_v2.iso`
- Windows edition: Windows 11 Pro, selected with Microsoft's generic install
  key for setup only; this does not activate Windows.
- VM resources: 4 CPUs, 8192 MiB RAM, 131072 MiB sparse disk
- Login user: `pax`
- Guest computer name: `pax-win-touch`
- Rust toolchain: `stable`
- Node.js: current LTS from winget
- WebAssembly helper: `wasm-pack` 0.15.0

The Windows ISO is not redistributed by this repo. Download the Windows 11
Arm64 ISO from Microsoft and either place it at the default path above or set
`PAX_FIRST_TOUCH_WINDOWS_ISO`. The script verifies the Microsoft-published
English ISO SHA256 unless `PAX_FIRST_TOUCH_WINDOWS_ISO_SHA256=skip` is set.

Set `PAX_FIRST_TOUCH_PASSWORD` to control the temporary VM password. If omitted,
the script generates one for the current invocation and prints it once. The
generated answer file and driver ISO are written under
`~/.cache/pax-first-touch/generated/`.

The first successful output should be a stopped and registered Parallels VM with
a `pax-first-touch-windows-workstation-prereqs` snapshot. The baseline installs
Parallels Tools, Visual Studio Build Tools with the C++ workload, Git for
Windows, Node.js LTS, Rust, the `wasm32-unknown-unknown` target, and
`wasm-pack`. On ARM64 Windows, the baseline explicitly installs the ARM64 VC
tools component so Rust's native MSVC linker is available. It also installs
Visual Studio's
Clang/LLVM compiler and MSBuild toolset because `wasm-pack`'s dependency graph
currently compiles `ring`, which expects `clang` on Windows Arm64. The baseline
adds the LLVM tool directory to the user's PATH and the smoke script imports the
MSVC environment before invoking Cargo.

Current Windows 11 25H2 Arm64 media may still show the OOBE license page in the
Parallels console even when the unattended `HideEULAPage` setting is present.
The provisioning script does not depend on completing that UI path: once
Parallels Tools can authenticate the `pax` user, the host uses `prlctl enter` to
start the prerequisite installer task as that user. If you want to use the VM as
an interactive desktop, open the console and accept the license page manually
after provisioning. The harness also removes the default sound device to avoid a
macOS microphone privacy prompt during this setup path.

To run the source-linked smoke against a running or stopped VM:

```sh
PAX_FIRST_TOUCH_PASSWORD=... scripts/first-touch/host/smoke-windows-source.sh
```

The smoke exposes a temporary read-only Parallels shared folder containing a
tarball of this checkout, installs `pax-cli` from source, creates a fresh
project outside the monorepo, patches `pax-kit` back to the synced source tree,
and builds the generated project for web.
