# Async Workbench

A proving fixture for [PAX-1013](../../../pax-docs/book/src/design/PAX-1013-application-async-spec.md). It uses the generated host and `#[application(WorkbenchApplication)]`, without a custom Rust `main` or application `@tick` polling.

Native setup registers an owned two-worker Tokio runtime through `pax-tokio`. Web uses `pax-web-async` and browser timers/fetch. Application services outlive panel remounts; node/operation scopes revoke stale results. Templates remain declarative.

## Run

From the repository root, using the repository's built `pax-cli`:

```sh
pax-cli run --path examples/src/async-workbench --target web
pax-cli run --path examples/src/async-workbench --target macos
pax-cli run --path examples/src/async-workbench --target ios --ios-device simulator
```

For the baked macOS application, use `pax-cli build --path examples/src/async-workbench --target macos --release`, then open the generated release app. iOS `run` also accepts `--release`. For Node-dependent commands on this workstation, run `source ~/.zshrc` and `nvm use` in the same shell. Omit redirected `CARGO_TARGET_DIR` for native CLI packaging.

## Setup and platform configuration

The [application services and Tokio guide](../../../pax-docs/book/src/event-handling-rust.md#application-setup)
explains the setup hooks, managed runtime, task lifetimes, and delivery APIs used here.

The fixture's `Cargo.toml` explicitly enables
`com.apple.security.network.client` and `com.apple.security.network.server`
under `[package.metadata.pax.macos.entitlements]`. Both are needed for its
loopback TCP server/client exercise inside the signed macOS sandbox. No new
network permissions are enabled globally for other Pax apps. See
[Apple property lists and entitlements](../../../pax-docs/book/src/targets-build-deploy.md#apple-property-lists-and-entitlements)
for typed inline values, source plist files, inheritance, and Apple's references.

These macOS sandbox keys do not apply to iOS. The fixture's native I/O is
loopback-only; an app that discovers or contacts LAN devices must configure its
own local-network usage description and any applicable discovery declarations.
Neither entitlements nor a Tokio runtime promise continued execution while an
iOS app is suspended.

## Scenarios

| Action | Expected result |
| --- | --- |
| Interact | Counter remains usable during a request. |
| Publish +1 | Application-owned shared data updates the panel's local computed label. |
| Burst +1000 | Native runs two finite workers with 500 atomic updates each; no updates are lost. Web performs the same publications locally. At most two burst workers run at once. |
| Run · 600 ms | Loading/Running becomes Ready/Completed after the executor timer. A small timeline indicator is mounted only while Running. |
| Fail safely | A controlled error appears after 600 ms; the application remains usable. The task itself completed normally with a Result error. |
| Replace A with B | B wins after 200 ms. A's publisher remains in independent work until 900 ms and logs that its stale write was rejected. |
| Cancel | Authority is revoked before Cancelled is displayed; queued callbacks and later guarded writes cannot overwrite it. |
| Progress burst | 1,000 guarded updates finish at 100%; intermediate property revisions may coalesce. |
| Stream 100 events | Capacity four is deliberately filled; the fifth immediate send reports Full. Awaited sends resume as the UI drains, and all 100 accepted events arrive in order. |
| Run local I/O | Native exchanges a fixed response over an ephemeral loopback TCP socket with a two-second timeout. Web fetches the bundled `sample.json`. |
| Local callback | An ordinary native thread completes into a UI-local closure that updates `LocalProperty`. Web queues the same callback locally. |
| Panic probe | Native reports the expected task panic through task status/error reporting. Web explains its abort-on-panic limitation; use Fail for a recoverable error. |
| Blocking probe | Native runs at most one blocking job with cooperative stop checks. Cancellation suppresses publication even if a job has already started. Web explains why blocking work is unavailable. |
| Unmount/Remount | Unmount discards the panel's local state and revokes pending results. Remount creates fresh panel state and scopes. The application-owned counter, history, and services remain; application starts stays one and panel mounts increases. |

Raw application-data publication is deliberately independent of task cancellation. A captured ordinary `Property` remains writable after panel removal. Use guarded publishers or `spawn_into` for results whose authority must expire.

The event history retains 100 records and displays the newest 12, with monotonic times and application/mount/operation identities. The shared-to-local label shows that binding's observed/published revisions. They are diagnostic counters, not a promise that each intermediate revision rendered. Native blocking and raw thread scenarios are bounded; no external service or account is required. Browser timer/fetch guards cancel their underlying resources when dropped.

Layout uses two columns at 900 px and above, stacked panels below, and full-width actions below 520 px. Setup/services are in `src/application.rs`, platform work in `src/work_service.rs`, and task/operation ownership in `src/request_panel.rs`.

## Qualification (2026-10-01)

| Check | Evidence |
| --- | --- |
| Core debug/release | Property, runtime, standard component, macro, manifest and Tokio unit suites pass. Runtime integration tests cover actual worker wake, structural unmount before callback dispatch, stale input targets and fatal UI callbacks. |
| Tokio variants | Owned and borrowed runtimes; native timers/TCP; dedicated continuously driven current-thread runtime with LocalSet/Rc state; cancellation, panic-on-abort, and started blocking-job shutdown pass in adapter tests. |
| Browser debug | Timer/error, interaction, replace, cancel, unmount/remount, progress burst, 100-event stream, local callback, shared burst and same-origin fetch verified. Phone/stacked/wide layouts inspected. |
| Browser wake-only | Timer and 100-event stream verified with recurring RAF and hidden-tab fallback disabled in a generated host copy. |
| Native debug | macOS rendering and native input verified: timer/error, TCP, callbacks, replacement/stale rejection, 100-event stream, task panic, blocking work, shared/progress bursts, in-flight cancellation and unmount/remount. The generated-cartridge C-ABI probe separately covers all scenarios and managed teardown without periodic ticks. |
| macOS display startup | Injected CoreVideo creation/start failures still allow initial rendering and timer delivery; display startup recovers after availability returns and a wake notification arrives. Debug and release host builds pass. This is a deterministic failure test, not a physical sleep/lock cycle. |
| Apple metadata | Typed inline and XML/binary source plists, date/data preservation, top-level replacement, iOS→iPadOS inheritance, platform isolation, managed-key diagnostics, and generated-host defaults pass in 16 metadata tests; 28 Apple packaging tests pass. |
| Native mobile | iOS simulator Rust + Swift debug and release builds pass; packaged plist booleans, arrays and dictionaries retain their types, and only debug receives the development-network usage description. Launch on the existing iPhone 17 Pro simulator was attempted, but the locked desktop prevented interaction and the native dev session did not become inspectable. Simulator/device interaction, background/resume and iPad behavior remain unverified. |
| Release fixture | Baked web build and timer/stream interactions pass. Universal macOS release build, native UI rendering/timer/stream/input, and the native C-ABI scenario suite pass. The final signature contains the fixture's opt-in network client/server entitlements; the signed release app now completes TCP loopback I/O, and its ordered 100-event stream and native input also pass. The headless C-ABI probe separately runs outside that app sandbox. The synchronous Increment release-web counter also renders and increments. |
| Reload | Browser Pax-only remount preserves application setup/data and revokes removed scopes. Web and actual macOS dylib replacement during a held request reset application state and leave the new application usable. A web configuration failure after managed registration leaves the old UI usable. Native C-ABI tests additionally cover candidate disposal and instance replacement. |

Run the reproducible wake-only and native-cartridge harnesses described in [the probe README](../../../tests/src/tokio-support-probe/README.md). Managed shutdown unit tests separately prove reverse dependency order, failed setup cleanup and retention after deadline expiry. Physical host close, suspension, or process termination cannot guarantee async cleanup.

macOS interactive checks were completed after unlocking the workstation. Mobile interaction and physical sleep/resume remain unverified; unexecuted targets and lifecycle paths are not claimed as passing. The memory probe reports approximately 382 retained bytes per local literal, 160 per unattached shared property, and 1,326 per shared property with a local projection on this workstation; these include bookkeeping and are not ABI/performance promises.
