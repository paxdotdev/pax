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
It also checks that the macro and packager dependency graphs exclude the runtime
renderer, so cross compilation does not build a host GPU stack for these tools.
It checks native app/dependency feature differences, wasm cross compilation,
library versus test units, a dependency build-script cfg, explicit rustc cfg,
release configuration, repeated runnable declarations, overlapping roots, and
dependency source changes. Each invocation inspects the generated manifest;
native tests also reconstruct the generated runtime manifest in debug and release.
On macOS, a C ABI probe loads the generated dylib, rejects missing/unknown root
names, creates two separate engines, and checks that each selected initializer
runs once even though their component definitions overlap.
The source-edit case restores its fixture file in a `finally` block.

A temporary external-workspace case gives the component library an unrelated
workspace member with an intentionally unresolvable dependency. Full offline
metadata from the library's workspace must fail, while a bare Cargo test of an
external consumer must succeed. No `PAX_*` environment handoff is provided. This
also exercises a custom library target name that differs from its package name.
It repeats the build with a renamed transitive dependency and after relocating
the library workspace without clearing the target cache. It then checks native
`init!`/`init_with!` for default/explicit selection, renamed UI dependencies,
custom library targets, same-package main, and a missing `cdylib` diagnostic.
These are compilation checks, not native UI launches. The temporary fixture is
removed after the check.

`app` renames its dependency on `widgets`. The widget's build script exists to
exercise a library-owned custom cfg. Widgets forward the configuration of `leaf`,
a transitive dependency whose component is declared with a qualified attribute
inside a private module, publicly re-exported, and uses a custom library path.
The consumer must not reevaluate either dependency's features or `cfg(test)`.
The leaf feature is enabled by its dependency edge, independently of app features.

The macro frontend reads only `cargo metadata --no-deps --offline` for its own
package declarations. Each compiled component library exports an internal helper
carrying its source identity, compiler-evaluated cfg facts, and component
dependency links. The consumer invokes that helper through Rust's dependency
path; it does not rediscover the dependency with another full Cargo resolution.
Helpers have package-qualified names to avoid collisions through glob imports.
The native initialization recipe uses the same discovery convention. Generated
code tracks the manifest file and directory environment to invalidate captured
paths when the source checkout moves.

Original sources are parsed once per compiler session. Completed discovery and
predicate answers are cached within that compilation unit. One deterministic
active runnable site emits the shared program catalog and ABI exports; repeated
roots share immutable definitions. Disabled source files are never opened.

This is a compiler acceptance fixture, not a rendering or native-launch test.
Arbitrary Rust macros that generate component declarations, `include!`, and
`#[path]` on inline modules are not supported by the source scanner. Multiple
versions of a component dependency and compiler source-path remapping need
separate qualification; the helper protocol is internal, not a public API.
