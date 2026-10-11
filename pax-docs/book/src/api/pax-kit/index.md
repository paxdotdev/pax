# pax-kit
<!-- summary: The application-facing Pax facade: components, properties, `runnable!`, and -->
<!-- tags: api, pax-kit -->

The application-facing Pax facade: components, properties, `runnable!`, and
platform hosting through one dependency.

Starters enable `standalone` (hosting) and `designtime` by default. On macOS,
`cargo run` runs the native application. `init!` and `init_with!` initialize
in the current native process, without parsing arguments or starting
development tools. Use `pax-cli` for serving, template reload, and platform
packaging/deployment. Macros compile the UI without an application build
script; merely building or depending on a UI library never launches it.

## Submodules
- [host](host.md)
