# Async Workbench

A fixture for application initialization, executor ownership, and scoped async work. It uses a real Apple Rust `main`, an explicit browser `init_web`, and `#[init(application::init)]` to construct the root without application `@tick` polling.

The native initializer constructs its own two-worker Tokio runtime and explicitly adopts it with `pax_tokio::Executor::owned`. Web uses `pax-web-async` and browser timers/fetch. Ordinary root-owned resources outlive panel remounts; node/operation scopes revoke stale results. Templates remain declarative.

## Run

From the repository root, using the repository's built `pax-cli`:

```sh
pax-cli run --path examples/fixtures/async-workbench --target web
pax-cli run --path examples/fixtures/async-workbench --target macos
pax-cli run --path examples/fixtures/async-workbench --target ios --ios-device simulator
```

For the baked macOS application, use `pax-cli build --path examples/fixtures/async-workbench --target macos --release`, then open the generated release app. iOS `run` also accepts `--release`. For Node-dependent commands on this workstation, run `source ~/.zshrc` and `nvm use` in the same shell. Omit redirected `CARGO_TARGET_DIR` for native CLI packaging.

## Setup and platform configuration

The [initialization and Tokio guide](../../../pax-docs/book/src/event-handling-rust.md#application-setup)
explains root construction, explicit runtime ownership, task lifetimes, and delivery APIs used here.

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

## Embedding and initialization

Initialization returns a Loading root immediately. Its staged timer changes the
phase to Ready after 600 ms; the host readiness observation does not wait for it.
`public/embedding.html` is a caller-owned browser host. It sends UTF-8 startup
bytes `7`, and the initializer adopts that as the application counter. The
Request increment button sends UTF-8 `increment` and displays the copied reply.
`read` returns the counter without changing it. Other payloads report an error.

The native caller-owned Swift fixture is in `tests/src/host-embedding` and uses
the same cartridge and host framework as standalone. Both fixtures keep host
controls alive during Pax work, reject a second mount, and expose explicit close
and remount.

## Scenarios

| Action | Expected result |
| --- | --- |
| Cartridge-owned input | Native/DOM text entry stays within the Pax region and can switch focus to the embedding host field. |
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
| Unmount/Remount | Unmount discards the panel's local state and revokes pending results. Remount creates fresh panel state and scopes. The application-owned counter, history, and resources remain; application starts stays one and panel mounts increases. |

Raw application-data publication is deliberately independent of task cancellation. A captured ordinary `Property` remains writable after panel removal. Use guarded publishers or `spawn_into` for results whose authority must expire.

The event history retains 100 records and displays the newest 12, with monotonic times and application/mount/operation identities. The shared-to-local label shows that binding's observed/published revisions. They are diagnostic counters, not a promise that each intermediate revision rendered. Native blocking and raw thread scenarios are bounded; no external service or account is required. Browser timer/fetch guards cancel their underlying resources when dropped.

Layout uses two columns at 900 px and above, stacked panels below, and full-width actions below 520 px. Initialization/resources are in `src/application.rs`, platform work in `src/work_service.rs`, and task/operation ownership in `src/request_panel.rs`.

## Checks

Run the reproducible wake-only and native-cartridge harnesses described in
[the probe README](../../../tests/src/tokio-support-probe/README.md). The native
embedding fixture and automated lifecycle probe live in
[`tests/src/host-embedding`](../../../tests/src/host-embedding/README.md).

Headless checks do not establish physical-device interaction or lifecycle
behavior. Physical host close, suspension, or process termination cannot
guarantee async cleanup.
