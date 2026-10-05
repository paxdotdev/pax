# Workspace cartridge qualification

This fixture keeps a normal Rust launcher in `host`, the selected cartridge in
`ui`, and a reusable component in `widgets`. It deliberately uses library target
names that differ from package names. `App` and `Counter` are public re-exports
from private modules. The dependency retains a legacy `#[main]` marker to check
that it behaves as an ordinary subtree when another cartridge selects the root.
`consumer` selects its own `Shell` and reuses the explicit UI cartridge's `App`.
Each UI library selects its root with `pax_kit::runnable!`.
The launcher also depends only on `pax-kit`, enabling `pax-kit/host` through
`standalone` and calling `pax_kit::init()?.run()`.
The UI library publicly re-exports `Counter`, so the consumer can name that
transitive component without depending directly on the widgets package.

From the repository root, after building `pax-cli`:

```sh
PAX_WORKSPACE_ROOT="$PWD" target/debug/pax-cli run \
  --path tests/src/cartridge-layouts/host --target web --libdev
PAX_WORKSPACE_ROOT="$PWD" target/debug/pax-cli build \
  --path tests/src/cartridge-layouts/host --target web --release --libdev
PAX_WORKSPACE_ROOT="$PWD" target/debug/pax-cli build \
  --path tests/src/cartridge-layouts/host --target macos --libdev
```

The workspace's Cargo output directory can be redirected with `CARGO_TARGET_DIR`;
`.pax/` always belongs to `host`. The compiler tests verify selection, qualified
root identity, dependency root flags, package-relative templates, both asset
directories, rich binary and release ProgramIR round trips, and Rust-manifest
generation. Negative tests cover missing roots, conflicting declarations,
non-component roots, nonexistent packages, and mismatched library targets.

For a live debug check, increment both buttons, edit `ui/src/ui/app.pax`, then
edit `widgets/src/counter.pax`. Both edits must reload from outside the host
directory. The root's counter survives template reload; rebuilt child instances
start with their defaults. Query `pax-cli dev inspect tree --path
tests/src/cartridge-layouts/host` to see `crate::ui::App` and
`layout_controls::Counter`. Both `assets/ui-mark.png` and
`assets/widget-mark.png` must be served. These fixture images reuse Space Game's existing RGBA PNG assets.

The initializer accepts an optional UTF-8 counter in startup data and handles
the `increment` host request. This lets the existing native lifecycle probe
exercise the same mount/ready/request/close/remount contract:

```sh
python3 tests/src/host-embedding/build.py \
  'tests/src/cartridge-layouts/host/.pax/build/debug/macos/app/Pax macOS (Development).app' \
  /tmp/pax-layout-probe.app macos probe
/tmp/pax-layout-probe.app/Contents/MacOS/HostEmbeddingProof
```

Use a fresh output path for each probe bundle. The fixture has no async executor;
Async Workbench remains the qualification example for executor ownership and
asynchronous teardown. Physical-device signing is not exercised by this fixture.
