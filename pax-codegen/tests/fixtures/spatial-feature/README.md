# Spatial capability contract

Run `python3 pax-codegen/tests/fixtures/spatial-feature/verify.py` from the
repository. Requires the `wasm32-unknown-unknown` Rust target and cached Cargo
dependencies. Checks that the same spatial template fails clearly without
`pax-kit/3d` and compiles with it, in debug and release-baked modes.

This fixture has a separate Cargo workspace so other spatial examples cannot
unify their feature into the disabled case. The host proc macro need not enable
`3d`: generated checks use the target runtime's capability.
