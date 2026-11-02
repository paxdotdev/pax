# Native export ownership fixture

This small, dependency-free fixture checks Rust/C linkage and Cargo package
composition. It does **not** use Pax's generated cartridge, engine, or native
hosts, and its miniature ABI is not the production Pax ABI.

Run from the repository root, using new directories for each run:

```sh
python3 pax-codegen/tests/fixtures/native-export-owner/verify.py \
  --work-dir /tmp/pax-native-export-owner-check \
  --output /tmp/pax-native-export-owner-evidence
```

The runner requires Python 3.9+, Cargo, the host Rust standard library, and
`nm`. It uses Cargo offline, one build job, and no external Rust dependencies.
Build products live under `--work-dir`, outside the repository. Each command's
output and a machine-readable `summary.json` are saved under `--output`.
Both directories must not already exist. The runner leaves its outputs for
inspection; remove its work directory when finished.

On macOS, Xcode's C and Swift compilers are also used. To additionally compile
and link clients for iOS devices and simulators, install the
`aarch64-apple-ios` and `aarch64-apple-ios-sim` Rust targets and add
`--apple-cross`. These checks use an iOS 15 deployment target and the installed
Xcode SDK. They do not boot a simulator, sign an app, or execute an iOS binary.

## What it checks

The generated Cargo workspace has a launcher, an adapter, and reusable UI
libraries. The website-like UI uses a movie library and two versions of a
calculator library through renamed dependencies. All UI libraries declare the
same Rust initializer name, with ordinary Rust symbol mangling. Only the
adapter exposes fixed C symbols. Its selected initializer may call a private
setup function within the chosen UI library.

The launcher uses a metadata-only macro from its adapter dependency, then
loads the adapter's dynamic library. The checks establish that:

- Ordinary `cargo run` builds the sidecar library as a normal dependency.
- UI components compose without running their libraries' initializers.
- Explicit Rust calls still reach each library's distinct initializer,
  including two versions of the same package.
- The adapter selects the intended program, rejects invalid selections before
  initialization, and owns engine allocation and deallocation.
- Two engine instances keep independent state; cleanup leaves no live engines.
- Each fixed C export occurs once in the adapter and is absent from the
  launcher's defined global symbols.
- Debug and release builds work, including release LTO, feature forwarding,
  warm Cargo builds, and relocation of the launcher with its sidecar.
- Another adapter can choose a dependency's program as its standalone root.
- C and Swift clients can link and call the static library on macOS.

Two negative controls guard the interpretation:

1. Giving both reusable libraries the same unmangled C initializer must fail
   either at link time or by violating initializer selection at runtime. On
   the validated macOS toolchain, it links but both calls reach one initializer.
2. A combined UI/launcher package cannot depend on an adapter that depends back
   on its UI library. Cargo rejects the package cycle even when the back edge
   disables default features.

## Coverage limits

The fixture has been exercised on macOS, with C and Swift cross-links for iOS
device and simulator targets. Windows and Linux host execution, actual Pax
macro generation, release binary baking, rendering, hot reload, application
packaging, and embedding in the real host frameworks are outside this test.
The two calculator versions share one model runtime; this does not qualify
mixing incompatible Pax runtime versions. Passing this fixture does not mark
the native website acceptance test as passing.
