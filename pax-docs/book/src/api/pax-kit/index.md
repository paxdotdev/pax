# pax-kit
<!-- summary: The application-facing Pax facade: components, properties, `runnable!`, and -->
<!-- tags: api, pax-kit -->

The application-facing Pax facade: components, properties, `runnable!`, and
platform hosting through one dependency.

Enable the `host` feature to use `pax_kit::init`, `pax_kit::init_with`, and
`pax_kit::host` on macOS, iOS and iPadOS.
Starter applications enable it through their `standalone` feature; UI library
and web builds leave it disabled. Build native applications with `pax-cli`
so the matching platform framework is linked and packaged.
Unsupported targets report a diagnostic here, so ordinary launchers need
only one `main` function without platform conditionals.
