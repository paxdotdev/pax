# Pax examples

Start with these nine examples. Each directory is a complete Rust package with
its own templates and assets; the adjacent `.rs` file also exposes it as a Cargo
example target.

| Example | Explore |
| --- | --- |
| [Calculator](calculator) | Graphing, native input, and two-axis scrolling |
| [Paxflix](paxflix) | App composition, nested scrolling, and runtime themes |
| [Living Quilt](living-quilt) | Expressive composition and interaction; the default CLI starter |
| [Ink & Light](ink-and-light) | A responsive postcard with local assets and Rust interaction |
| [Increment](increment) | A minimal template-to-Rust interaction |
| [Pax Logo](pax-logo) | Compact declarative motion and a reusable animated component |
| [Liquid Glass](liquid-glass) | Apple native materials and the fallback on other targets |
| [Materials](materials) | Lighting and material composition |
| [Viewport Proximity](viewport-proximity) | Viewport events, data loading, and motion |

From the repository root, run a web example on macOS, Linux, or Windows:

```sh
pax-cli run --path examples/living-quilt --target web
```

For direct native execution on macOS:

```sh
cargo run -p living-quilt
cargo run --example living-quilt
```

Prefer `-p <name>` when iterating: the root package shares all example
dev-dependencies across `--example` targets. Direct Cargo execution does not
start template hot reload; use `pax-cli run` for a development session.
See [targets and prerequisites](../pax-docs/book/src/targets-build-deploy.md).

[Fixtures and supporting examples](fixtures/README.md) contain focused teaching
studies, platform probes, regression cases, and historical experiments. They
remain runnable and available through `pax-cli docs examples <name>`.

CLI starter eligibility is separate: [the starter registry](bundled-cli-examples.toml)
continues to bundle Living Quilt, Ink & Light, and Increment only. Keeping an
example here does not add it to every CLI installation.

To contribute a supporting example, run `examples/create-example.sh <name>`.
Use `--showcase <name>` only for an agreed addition to this curated list, and
update the relevant index. The helper registers the package and its Cargo example
target. After source changes, run `python3 scripts/sync-docs-examples.py`; for a
bundled starter, also run `python3 scripts/sync-cli-examples.py`.
