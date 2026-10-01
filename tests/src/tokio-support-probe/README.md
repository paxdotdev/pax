# PAX-1013 — Executor and property probes

These headless probes accompany the [application/async spec](../../../pax-docs/book/src/design/PAX-1013-application-async-spec.md). They establish individual ownership and executor contracts; they do not establish complete Tokio application support.

## Current property model

`Property<T>` owns synchronized shared snapshots. `LocalProperty<T>` and untyped dependency handles stay on the UI thread. Workers can clone, read, update and drop shared properties; the owning graph imports revisions and evaluates local bindings. Intermediate publications coalesce.

The original investigation at repository base `090348859` found that a bare thread-local slot-map handle was accidentally transferable. Moving it to a worker could read an unrelated value (`99` instead of `7`). `property_thread_diagnostic` now asserts the corrected behavior: the worker reads `7`, publishes `21`, and the owner's computed value becomes `42` after import. The old incorrect value is not an expected result.

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

The probe measures local/shared reads and writes, atomic updates, 100-dependent fan-out, coalesced bursts, and allocation cost. On this Apple Silicon workstation, a 2,000-property optimized batch retained approximately:

| Representation | Bytes per property | Allocations per property |
| --- | ---: | ---: |
| Local literal | 382 | 1 |
| Shared, unattached | 160 | 2 |
| Shared with local projection | 1,326 | 14.03 |

Both handles are eight bytes. These figures include amortized table/attachment bookkeeping, exclude allocator metadata, and are diagnostic observations rather than a stable ABI or performance promise. The counting allocator itself adds overhead to the reported timings; do not compare those timings to runs made before instrumentation. Allocation cost is a reason to keep engine/layout bookkeeping local. Measure actual applications before making a frame-rate claim.

## Application proof and remaining qualification

[Async Workbench](../../../examples/src/async-workbench/README.md) uses the normal generated host and `#[application(...)]` setup hook. It now exercises managed Tokio/browser tasks, cancellation, guarded publication, bounded callback delivery, local I/O and asynchronous shutdown. The focused `pax-tokio` tests also cover owned/borrowed runtimes, a dedicated current-thread/LocalSet service, panic observation, and non-abortable blocking work.

The root remains a cartridge library. Native Swift and browser JavaScript own platform startup and frame scheduling; a CLI launcher with `#[tokio::main]` would run in a separate process. A native runtime needs an explicit application owner and continuously driven executor. A `Handle` does not keep its runtime alive. Browser Wasm uses browser-compatible futures and does not gain desktop Tokio sockets, filesystem or timers.

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
cargo run --manifest-path tests/src/tokio-support-probe/Cargo.toml --bin native_cartridge_probe -- examples/src/async-workbench/target/aarch64-apple-darwin/debug/deps/libasync_workbench.dylib
```

After a debug web build, `python3 tests/src/tokio-support-probe/prepare-wake-only-web.py`
creates ignored host copies with recurring RAF and hidden-tab polling disabled.
Open `/wake-only.html` on the fixture's dev server. Allow initial Pax reload to
settle, then run the timer and stream without further input. Both should update
through the host wake routes. The normal app still uses its normal frame loop.

These headless checks do not establish native visual/input, physical iOS,
background/resume or platform logic-reload behavior. See the fixture README for
the qualification matrix and remaining limits.

## macOS display-clock startup failure

`macos-display-link-failure.m` injects failure into the generated Swift host's
CoreVideo creation or start call. It tests startup/wake recovery without locking
or sleeping the workstation. Build the local fixture first, quit its existing
process, then run from the repository root:

```sh
clang -dynamiclib -framework CoreVideo -framework AppKit tests/src/tokio-support-probe/macos-display-link-failure.m -o /tmp/pax-display-link-failure.dylib
touch /tmp/pax-display-link-block
DYLD_INSERT_LIBRARIES=/tmp/pax-display-link-failure.dylib PAX_DISPLAY_LINK_PROBE_BLOCK=/tmp/pax-display-link-block PAX_DISPLAY_LINK_PROBE_STAGE=create 'examples/src/async-workbench/.pax/build/debug/macos/app/Pax macOS (Development).app/Contents/MacOS/Pax macOS (Development)' > /tmp/pax-display-link-probe.log 2>&1 &
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
