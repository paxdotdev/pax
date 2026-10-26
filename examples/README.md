# Pax examples

Start with these ten examples. Each directory is a complete Rust package with
its own templates and assets; the adjacent `.rs` file also exposes it as a Cargo
example target.

| Example | Explore |
| --- | --- |
| [Calculator](calculator) | Graphing, native input, and two-axis scrolling |
| [LED Oscilloscope](led-oscilloscope) | A discrete RGB marquee, batched drawing, and tactile knobs |
| [Paxflix](paxflix) | App composition, nested scrolling, and runtime themes |
| [Living Quilt](living-quilt) | Expressive composition and interaction |
| [Ink & Light](ink-and-light) | A responsive postcard with local assets and Rust interaction |
| [Increment](increment) | A minimal template-to-Rust interaction |
| [Pax Logo](pax-logo) | The default CLI starter: a standard-library logo, slider, and buttons |
| [Liquid Glass](liquid-glass) | Apple native materials and the fallback on other targets |
| [Materials](materials) | Lighting and material composition |
| [Viewport Proximity](viewport-proximity) | Viewport events, data loading, and motion |

From the repository root, run a web example on macOS, Linux, or Windows:

```sh
pax-cli run --path examples/pax-logo --target web
```

For direct native execution on macOS:

```sh
cargo run -p pax-logo
cargo run --example pax-logo
```

Prefer `-p <name>` when iterating: the root package shares all example
dev-dependencies across `--example` targets. Direct Cargo execution does not
start template hot reload; use `pax-cli run` for a development session.
See [targets and prerequisites](../pax-docs/book/src/targets-build-deploy.md).

[Fixtures and supporting examples](fixtures/README.md) contain focused teaching
studies, platform probes, regression cases, and historical experiments. They
remain runnable and available through `pax-cli docs examples <name>`.

CLI starter eligibility is separate: [the starter registry](bundled-cli-examples.toml)
bundles Pax Logo, Living Quilt, Ink & Light, and Increment. Keeping an
example here does not add it to every CLI installation.

To contribute a supporting example, run `examples/create-example.sh <name>`.
Use `--showcase <name>` only for an agreed addition to this curated list, and
update the relevant index. The helper registers the package and its Cargo example
target. After source changes, run `python3 scripts/sync-docs-examples.py`; for a
bundled starter, also run `python3 scripts/sync-cli-examples.py`.
