# Pax first-touch workstation harness

This directory holds reproducible local VM setup for validating the public Pax
first-touch flow outside the Pax monorepo. The harness should create a clean
workstation baseline and then run smoke scenarios that use the same commands a
new user would use.

The harness intentionally keeps OS provisioning separate from Pax validation:

- `host/` scripts run on the macOS workstation and create/configure VMs.
- `guest/` files are consumed inside the VM during install or smoke runs.

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
- WebAssembly helper: `wasm-pack` 0.15.0

Set `PAX_FIRST_TOUCH_PASSWORD` to control the temporary VM password. If omitted,
the script generates one for the current invocation and passes it to Packer via
environment variables.

The first successful output should be a stopped and registered Parallels VM with
a `pax-first-touch-ubuntu-workstation-prereqs` snapshot. Pax smoke scripts
should run against that baseline or a clone/snapshot of it.

The baseline intentionally includes the Linux development packages needed by
Pax's current web build graph: GLib, Cairo, and Pango headers. Without these,
source-built `pax-cli` can fail before a user reaches a generated app.

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
Windows, Rust, the `wasm32-unknown-unknown` target, and `wasm-pack`. On ARM64
Windows, the baseline explicitly installs the ARM64 VC tools component so
Rust's native MSVC linker is available. It also installs Visual Studio's
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
