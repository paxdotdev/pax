# Pax compiler

`pax-compiler::perform_build` coordinates source preparation, platform compilation,
resource assembly and packaging for `pax-cli`. The supported backends are web,
macOS, iOS and iPadOS. Native applications retain a separate cartridge ABI;
web cartridges compile to Wasm.

## Source analysis and generation

The shared implementation lives in [`pax-codegen`](../pax-codegen/README.md):

1. Cargo expands Pax macros, which analyze the Rust source selected by each
   owning compiler and parse templates into a `PaxManifest`. The CLI requests a
   manifest sidecar from this same compilation. It does not execute application
   code or compile a parser binary.
2. Cartridge generation connects the definitions to Rust component types and
   handlers. Development builds use rich manifest data; release builds embed
   the program as Rust constructors. PAXEL expressions remain program data for
   the runtime evaluator.
3. This crate prepares the platform interface, compiles the application and
   assembles its resources. Its build and development-server responsibilities
   are separate from the source-analysis library.

`pax_compiler::static_analysis` and `pax_compiler::svg_import` remain re-exports,
so existing compiler clients keep their source paths. Ordinary macOS `cargo run`
and repository `cargo run --example <name>` compile
Pax through the macros without an application build script. `pax-host` builds
the generic macOS SDK; this crate packages the same native sources for Apple
bundles and manages web serving and development sessions. The implementation
history lives in the [phase-three specification](../pax-docs/book/src/design/PAX-1015-main-entry-spec.md).
