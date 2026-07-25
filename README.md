# Pax

**A language-first GUI framework for Rust.**

Pax combines declarative `.pax` templates and spreadsheet-like PAXEL
expressions with Rust application logic. Its portable runtime supports
GPU-rendered graphics, native controls, responsive layout, animation, and
compositing across web and native Apple targets.

Pax is open source and ready for builders. The framework, compiler, runtime,
CLI, hot-reload system, and scriptable developer tools all live in this
repository.

## Get started

Install the CLI, create a project, and run it:

```sh
cargo install pax-cli
pax-cli create my-first-project
cd my-first-project
pax-cli run
```

See the [getting-started guide](https://docs.pax.dev/) for workstation setup and
target-specific prerequisites.

## Authoring model

- `.pax` templates declare UI structure, layout, styling, bindings, control
  flow, and motion.
- PAXEL expressions derive reactive values without side effects.
- Rust owns application state, event handlers, platform integration, and other
  side effects.
- `pax-cli run` provides hot reload during development.
- `pax-cli dev` exposes screenshots, scene inspection, event driving, logs, and
  source-aware mutation for human and agent workflows.

## Targets

Pax builds WebAssembly applications for the web and native applications for
macOS, iOS, and iPadOS. See the documentation for the current support matrix
and platform-specific setup.

## Examples and documentation

- [Documentation](https://docs.pax.dev/)
- [Examples](examples/src)
- [Contribution guide](CONTRIBUTING.md)
- [Community Discord](https://discord.com/invite/Eq8KWAUc6b)

## License

© PaxCorp Inc.

Pax is licensed under either the [MIT license](LICENSE-MIT) or the
[Apache License 2.0](LICENSE-APACHE), at your option.
