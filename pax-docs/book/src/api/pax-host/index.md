# pax-host
<!-- summary: Apple process entry and mounting across a separate cartridge ABI. -->
<!-- tags: api, pax-host -->

Apple process entry and mounting across a separate cartridge ABI.
Applications use `pax_kit::init!` and `pax_kit::init_with!`; options, errors,
and embedding types live in `pax_kit::host`. Enable `pax-kit/host` without
adding a direct `pax-host` dependency. The SDK is built on demand for native
development. The shared compiler provides bundles and iOS deployment through
either the Cargo development entry or the CLI.

```ignore
fn main() -> Result<(), pax_kit::host::Error> {
    pax_kit::init!()?.run()
}
```

## Submodules
- [native](native.md)

## Structs
### `Error`
An initialization, development launch, or mounting failure.

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
