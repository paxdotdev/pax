# pax-kit
<!-- summary: The application-facing Pax facade: components, properties, `runnable!`, and -->
<!-- tags: api, pax-kit -->

The application-facing Pax facade: components, properties, `runnable!`, and
platform hosting through one dependency.

Starters enable `standalone` (hosting) and `designtime` by default. Ordinary
`cargo run` launches web development; `cargo run -- pax --target=ios` selects
an Apple target on a Mac. The shared packager needs no installed Pax CLI or
application build script. `init_with!` always initializes in the current
native process for application-owned startup and embedding. Merely building
or depending on a UI library never launches it.

## Submodules
- [host](host.md)
