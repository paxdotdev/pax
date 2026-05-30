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
