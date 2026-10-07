# Pax code generation

Shared source analysis and Rust generation for Pax's Cargo and CLI workflows.
This crate contains no native SDK builder, dev server, watcher or rendering
engine. It reads definitions without calling user initializers or mounting UI.

The production `pax-macro` frontend asks rustc to select `cfg` predicates in each
owning compilation unit. Component libraries export hidden configuration helpers,
including transitive dependencies. `configured_source` applies those answers
before opening conditional modules or interpreting fields and templates.
`build_program_catalog_from_sources` consumes the selected ASTs; missing
configuration is a request to the frontend, never permission to scan raw files.

The catalog analyzes the union of declared runnable roots. Shared definitions
are visited once; `ProgramCatalog::manifest_for` selects a root without repeating
analysis. Declaration scopes and initializer paths remain source metadata. The
macro validates initializer agreement, emits one set of platform exports, and
invokes the selected initializer only when the host prepares that root.
Definitions are shared; engine instances, properties, and async lifetimes are not.
The native host currently permits one live mount at a time.

`generate` returns Rust in memory, using rich development data or baked release
constructors. The CLI obtains its manifest from the same macro compilation, then
packages the platform interface and resources. Raw-source `build_manifest` and
metadata-based scanner APIs remain available for compiler tooling and tests;
those APIs do not evaluate Rust cfgs themselves. Production builds use selected
sources instead.

Web adapters have library-specific wasm-bindgen export names derived from Cargo
package and target identity. The packaging step aliases the selected export to
`init_web` in JavaScript and TypeScript; it does not choose by export order or
`CARGO_PRIMARY_PACKAGE`. This permits runnable dependencies in the same wasm
module, including when multiple workspace packages are primary Cargo targets.

Source analysis uses file spans to retain inline Pax source exactly. The macro
frontend scopes `proc-macro2`'s fallback parser around source analysis and restores
compiler spans afterward, preserving caller hygiene at its callbacks. Parsed
sources, completed discoveries, predicate answers, and generated programs are
cached within a compilation unit. Helper names include the owning library entry
identity to avoid collisions through glob re-exports.

The [configuration-handoff fixture](tests/fixtures/cfg-handoff/README.md) tests
production macros, native/wasm/debug/release configuration, renamed and transitive
dependencies, source edits, overlapping roots, and native initialization through
the C ABI. It records the scanner's remaining scope limits. Arbitrary Rust macros
that generate component declarations are not a supported source-discovery path.
