# pax-host
<!-- summary: Native process entry and mounting across a separate cartridge ABI. -->
<!-- tags: api, pax-host -->

Native process entry and mounting across a separate cartridge ABI.
Applications use `pax_kit::init!` and `pax_kit::init_with!`; options, errors,
and embedding types live in `pax_kit::host`. Enable `pax-kit/host` without
adding a direct `pax-host` dependency. Cargo builds the SDK for native hosting.
The CLI provides bundles, development servers, and iOS deployment;
initialization itself stays in the calling native process.

Direct Windows Cargo launchers stage embedded TTF/OTF/TTC resources into a
verified cache beside the executable for WinUI resource loading. This needs a
writable executable directory with hard-link support; packaged applications
use their existing copied assets and do not extract fonts at startup. No
compiler or SDK build tool runs during initialization.
Direct desktop Cargo launches also verify that the separately built cdylib
matches the launcher's generated program identity. Rebuild both artifacts
after Rust changes; a stale cartridge is rejected before initialization.
Windows CLI packages merge Windows 10/11 compatibility and PerMonitorV2 DPI
declarations into the launcher EXE. Direct Cargo applications must supply
their own executable manifest for OS compatibility. The host establishes
PerMonitorV2 awareness before creating UI and rejects incompatible awareness
already established by the application.

```ignore
fn main() -> Result<(), pax_kit::host::Error> {
    pax_kit::init!()?.run()
}
```

## Submodules
- [native](native.md)

## Structs
### `Error`
An initialization, native application loop, or mounting failure.

---

### `InitOptions`
Copied application inputs for the separately linked cartridge.

#### Properties
##### `startup_data`
Type: `Vec`<`u8`>

Application-defined encoding, at most 64 KiB. The root initializer reads
these bytes through `InitContext::startup_data()`.

##### `entry`
Type: `Option`<`String`>

Canonical root path from `runnable!`; required for multiple distinct roots.
