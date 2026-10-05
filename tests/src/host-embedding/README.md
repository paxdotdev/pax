# Caller-owned native host fixture

This fixture links the same `PaxHost` framework and Async Workbench cartridge as
the standalone application, but supplies its own SwiftUI process entry, window,
view/controller and controls. The production launcher is Rust; the fixture is
intentionally a different caller exercising the public C ABI.

Build the cartridge/framework with the repository CLI first:

```sh
PAX_WORKSPACE_ROOT="$PWD" target/debug/pax-cli build --path examples/src/async-workbench --target macos --libdev
python3 tests/src/host-embedding/build.py \
  'examples/src/async-workbench/.pax/build/debug/macos/app/Pax macOS (Development).app' \
  /tmp/pax-host-embedding.app
```

Open the resulting app. The script refuses to replace an existing output. For a
mobile simulator, build with `--target ios` or `--target ipados` and an existing
`--ios-device simulator:<UDID>`; pass the resulting `.app` to the same script
with a final `ios-simulator` argument. Install and launch it with `simctl` on
that destination. It is not a device/distribution signing script.

The fixture copies build artifacts, compiles its own executable, then signs that
fixture bundle. Production packaging instead copies Cargo's executable through
an Xcode build phase before Xcode's normal signing; this script is not that path.
The ABI header is `pax-host/include/pax_host.h`. All calls and observer callbacks
run on the main thread. The caller retains callback contexts until release/reply.

Check these behaviors:

- Mount reports Ready. Startup bytes `7` become the cartridge's initial counter.
  Request increment reports `8` and updates its rendered state.
- Host and cartridge text fields can alternate focus. Both host and Pax counters
  respond during the 600 ms timer and stream. Resize changes only the Pax region.
- Collapse before mounting still reaches Ready and answers requests. Expanding
  restores the existing state; closing while collapsed still acknowledges cleanup.
- Try second mount rejects the request. The existing region remains interactive.
- Close removes Pax, reports cleanup acknowledgement, and leaves the host text,
  counter and window intact. Mount again creates a fresh cartridge state.
- With no mount, Fail initialization supplies malformed counter bytes. It reports
  failure and retires the already-created executor. The host loop continues
  driving cleanup; a new prepare is permitted after acknowledgement.
- Closing or removing the region during work cancels its authority. The fixture's
  representable dismantle closes the session; retained cleanup outlives the view.

The browser equivalent is Async Workbench's `public/embedding.html`. It also
exposes startup cancellation, hide/show, resizing, request/reply and failed init.
These are single-region fixtures. They do not claim multiple simultaneous Pax
mounts, physical-device lifecycle qualification, or new native-control features.

For an automated ABI/lifecycle regression, add `probe` after the platform:

```sh
python3 tests/src/host-embedding/build.py \
  'examples/src/async-workbench/.pax/build/release/macos/app/Pax macOS (Release).app' \
  /tmp/pax-host-lifecycle-probe.app macos probe
/tmp/pax-host-lifecycle-probe.app/Contents/MacOS/HostEmbeddingProof
```

On a simulator, use `ios-simulator probe`, install the result, and run
`xcrun simctl launch --console <UDID> dev.pax.host-lifecycle-probe`. The test
exits with status 0 and prints `PASS native host lifecycle probe` after failed
initialization cleanup, initial zero-height readiness, request/reply, expansion,
close acknowledgement, and fresh-state replacement followed by collapsed close.
A 15-second deadline fails the test. It drives no engine/shutdown pump itself.
This supplements the interactive fixture; it does not test visual appearance,
keyboard/IME, accessibility, or physical-device lifecycle.
