# Compiler configuration handoff acceptance fixture

This separate Cargo workspace exercises the production `pax_kit::pax` and
`runnable!` macros, source analysis, and generated cartridge descriptors. It has
no application build script or fixture-only macro implementation.

Run from the repository root with stable Rust and the wasm32 target installed:

```sh
python3 pax-codegen/tests/fixtures/cfg-handoff/verify.py \
  --target-dir /private/tmp/pax-cfg-handoff-build
```

The verifier uses offline Cargo resolution; dependencies must already be cached.
It checks native app/dependency feature differences, wasm cross compilation,
library versus test units, a dependency build-script cfg, explicit rustc cfg,
release configuration, repeated runnable declarations, overlapping roots, and
dependency source changes. Each invocation inspects the generated manifest;
native tests also reconstruct the generated runtime manifest in debug and release.
On macOS, a C ABI probe loads the generated dylib, rejects missing/unknown root
names, creates two separate engines, and checks that each selected initializer
runs once even though their component definitions overlap.
The source-edit case restores its fixture file in a `finally` block.

`app` renames its dependency on `widgets`. The widget's build script exists to
exercise a library-owned custom cfg. Widgets forward the configuration of `leaf`,
a transitive dependency whose component is declared with a qualified attribute
inside a private module, publicly re-exported, and uses a custom library path.
The consumer must not reevaluate either dependency's features or `cfg(test)`.
The leaf feature is enabled by its dependency edge, independently of app features.

Original sources are parsed once per compiler session. Completed discovery and
predicate answers are cached within that compilation unit. One deterministic
active runnable site emits the shared program catalog and ABI exports; repeated
roots share immutable definitions. Disabled source files are never opened.

This is a compiler acceptance fixture, not a rendering or native-launch test.
Arbitrary Rust macros that generate component declarations, `include!`, and
`#[path]` on inline modules are not supported by the source scanner. Multiple
versions of a component dependency and compiler source-path remapping need
separate qualification; the helper protocol is internal, not a public API.
