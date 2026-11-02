# Generated manifest constructor acceptance

This standalone fixture compiles the actual expression emitted by
`rust_manifest::to_rust_expression`, including a renamed engine path. Its build
script checks deterministic output and private non-inlined helper boundaries.
The runtime tests compare all serialized fields with the original manifest,
check empty collections, and mutate one instance to verify independence.
Two expressions share one function scope to catch leaked helper names. The
fixture includes overlapping component names, ordered template children,
comments and source locations, expressions and dependencies, nested values,
handlers, timelines, route metadata, type descriptors, and asset roots.

From the repository root, run both configurations with cached dependencies:

```sh
cargo test --offline --manifest-path pax-manifest/tests/fixtures/rust-manifest/Cargo.toml --jobs 1
cargo test --offline --manifest-path pax-manifest/tests/fixtures/rust-manifest/Cargo.toml --jobs 1 --release
```

Set `CARGO_TARGET_DIR` to reuse an existing validation target directory. The
fixture uses the compiler only as a build dependency and does not launch a GPU
or platform host. It verifies constructor semantics, not build performance or
native rendering. Measure those separately with a representative application.
