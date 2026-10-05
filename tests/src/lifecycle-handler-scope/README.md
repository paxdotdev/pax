# PAX-1018 regression fixture

Run using this checkout's CLI from the monorepo root:

```sh
cargo run -p pax-cli -- run --path tests/src/lifecycle-handler-scope --target web
```

Build the release fixture and serve the emitted bundle:

```sh
cargo run -p pax-cli -- build --path tests/src/lifecycle-handler-scope --target web --release
python3 -m http.server 62318 --bind 127.0.0.1 --directory tests/src/lifecycle-handler-scope/.pax/build/release/web
```

Both owners should initially show Group and Child inline counts `1 / 0`,
`yes / yes / yes` for tick/pre-render/Stacker pre-render, and Group context
width `240`. The child itself should show `Child settings mount: 1`.

Click **Unmount children**: both pairs of counts become `1 / 1` and the
child panels disappear. Click **Mount children**: counts become `2 / 1`,
and the recreated child state again shows `Child settings mount: 1`.
Repeat to check independent owner instances and complete child lifetimes.

In debug, edit the title in `lib.pax` while the app runs to exercise template
reload, then repeat the toggle. Reload may recreate component state; counts
should follow the newly mounted lifetime without a downcast panic.

The library suite additionally asserts parent/child state isolation, target
context identity, partial/full reload, render reparenting, suspension,
subscription cleanup, final owner release, owner-scoped reactive bindings,
and the mounted lifecycle of component mask sources:

```sh
cargo test -p pax-runtime --test lifecycle_handler_scope
```
