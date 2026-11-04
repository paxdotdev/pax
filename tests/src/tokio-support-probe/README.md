# PAX-1013 — Executor and property probes

These headless probes accompany the application/async spec archived in [PAX-1013](https://linear.app/paxdev/issue/PAX-1013). They establish individual ownership and executor contracts; they do not establish complete Tokio application support.

## Current property model

`Property<T>` owns synchronized shared snapshots. `LocalProperty<T>` and untyped dependency handles stay on the UI thread. Workers can clone, read, update and drop shared properties; the owning graph imports revisions and evaluates local bindings. Intermediate publications coalesce.

`property_thread_diagnostic` checks that a worker reads `7`, publishes `21`,
and the owner's computed value becomes `42` after importing the update.

The library tests additionally exercise a multithread Tokio timer, a continuously driven current-thread runtime/LocalSet, an undriven current-thread runtime, explicit cancellation and closed-channel behavior. Their `block_on`, joins and channel receives are test synchronization, not instructions to block a Pax event handler. Compile-fail examples reject moving local properties into native Tokio tasks.

## Run

From the repository root, with cached dependencies if using `--offline`:

```sh
cargo test --manifest-path tests/src/tokio-support-probe/Cargo.toml
cargo run --manifest-path tests/src/tokio-support-probe/Cargo.toml --bin property_thread_diagnostic
cargo run --release --manifest-path tests/src/tokio-support-probe/Cargo.toml --bin property_cost_probe
```

Repeat correctness checks with `--release`. An explicit `CARGO_TARGET_DIR` can keep this standalone workspace's artifacts outside the checkout.

## Cost probe

The probe measures local/shared reads and writes, atomic updates, 100-dependent
fan-out, coalesced bursts, and allocation cost. Its counting allocator adds
measurement overhead; compare equivalent builds and workloads. The results are
diagnostic observations, not stable ABI or performance guarantees.

## Application fixture

[Async Workbench](../../../examples/fixtures/async-workbench/README.md) uses the normal generated host and `#[init(path)]` typed root initializer. It now exercises explicit Tokio/browser executors and scoped tasks, cancellation, guarded publication, bounded callback delivery, local I/O and asynchronous shutdown. The focused `pax-tokio` tests also cover owned/borrowed runtimes, a dedicated current-thread/LocalSet service, panic observation, and non-abortable blocking work.

The root remains a cartridge library. The standalone Apple executable has a real Rust `main` and calls the SwiftUI host bridge; browser JavaScript calls `init_web`. Frame scheduling remains with the host. The launcher and cartridge are distinct linkage domains, so runtime handles must not cross that ABI. A native runtime needs an explicit application owner and continuously driven executor. A `Handle` does not keep its runtime alive. Browser Wasm uses browser-compatible futures and does not gain desktop Tokio sockets, filesystem or timers.

Primary executor references:

- [Tokio: bridging synchronous and asynchronous code](https://tokio.rs/tokio/topics/bridging)
- [Tokio 1.53.1 Wasm support](https://docs.rs/tokio/1.53.1/tokio/#wasm-support)
- [LocalSet](https://docs.rs/tokio/1.53.1/tokio/task/struct.LocalSet.html)
- [Runtime ownership and shutdown](https://docs.rs/tokio/1.53.1/tokio/runtime/struct.Runtime.html)
- [Browser spawn_local](https://wasm-bindgen.github.io/wasm-bindgen/api/wasm_bindgen_futures/fn.spawn_local.html)

`native_cartridge_probe` loads a built macOS Async Workbench dylib and drives its
real C ABI without a display link. It checks timers, replacement, 100-event
backpressure, TCP I/O, local callbacks, progress, cancellation, atomic updates,
remount, task panic, blocking work and managed teardown. It advances frames only
for input or a host wake. Run it with:

```sh
cargo run --manifest-path tests/src/tokio-support-probe/Cargo.toml --bin native_cartridge_probe -- examples/fixtures/async-workbench/target/aarch64-apple-darwin/debug/deps/libasync_workbench.dylib
```

After a debug web build, `python3 tests/src/tokio-support-probe/prepare-wake-only-web.py`
creates ignored host copies with recurring RAF and hidden-tab polling disabled.
Open `/wake-only.html` on the fixture's dev server. Allow initial Pax reload to
settle, then run the timer and stream without further input. Both should update
through the host wake routes. The normal app still uses its normal frame loop.

`native_keyboard_probe` uses a built macOS Calculator cartridge to check the
Swift keyboard wire fixture, default prevention, key down/up/press dispatch and
`6+2=8`. Run the Swift encoder/focus regressions too; together these catch wire
formats that round-trip within Swift but fail Rust's modifier-enum decoder.

```sh
swift test --package-path pax-host/native/common/pax-swift-common --scratch-path /tmp/pax-keyboard-tests --filter PaxKeyboardTests
cargo run --manifest-path tests/src/tokio-support-probe/Cargo.toml --bin native_keyboard_probe -- 'examples/calculator/.pax/build/debug/macos/app/Pax macOS (Development).app/Contents/Frameworks/PaxCartridge.framework/PaxCartridge'
```

Repeat against the corresponding release cartridge after a `pax-cli build
--target macos --release`. The probe itself can stay unoptimized: it is the
loaded cartridge whose baked event-handler path must be checked. Real native
input and embedding focus still need app-level checks.

These headless checks do not establish native visual/input, physical iOS,
or background/resume behavior. Rust logic replacement is no longer supported.
See the fixture README for interactive scenarios and run instructions.

## macOS display-clock startup failure

`macos-display-link-failure.m` injects failure into the generated Swift host's
CoreVideo creation or start call. It tests startup/wake recovery without locking
or sleeping the workstation. Build the local fixture first, quit its existing
process, then run from the repository root:

```sh
clang -dynamiclib -framework CoreVideo -framework AppKit tests/src/tokio-support-probe/macos-display-link-failure.m -o /tmp/pax-display-link-failure.dylib
touch /tmp/pax-display-link-block
DYLD_INSERT_LIBRARIES=/tmp/pax-display-link-failure.dylib PAX_DISPLAY_LINK_PROBE_BLOCK=/tmp/pax-display-link-block PAX_DISPLAY_LINK_PROBE_STAGE=create 'examples/fixtures/async-workbench/.pax/build/debug/macos/app/Pax macOS (Development).app/Contents/MacOS/Pax macOS (Development)' > /tmp/pax-display-link-probe.log 2>&1 &
pax_probe_pid=$!
```

The UI must render despite the rejection logged in `/tmp/pax-display-link-probe.log`.
Run the timer and Interact; the result and counter must update. Then restore
clock availability and deliver the test-only screens-did-wake notification:

```sh
rm /tmp/pax-display-link-block
kill -USR1 "$pax_probe_pid"
```

The log must show the wake notification followed by successful creation/start
(`returned 0`). Quit the fixture normally and repeat with
`PAX_DISPLAY_LINK_PROBE_STAGE=start`. This shim is only for the unsigned local
test build; it is not part of the generated host or shipped application.
