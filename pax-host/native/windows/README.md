# WinUI desktop host

This experimental host connects a user-owned Rust `main`, a Pax desktop cartridge,
real WinUI controls, and the shared Pax GPU renderer. It is a standalone host;
embedding through `mount` returns an explicit unsupported error. One WinUI
`Application.Start` is supported per process.

The host establishes per-monitor-v2 DPI awareness before creating UI and rejects
an incompatible awareness mode already selected by the application. An
executable manifest is preferred for packaged launchers; direct Cargo launchers
also need the Windows App SDK's supported-OS declaration as part of their app
configuration.

`pax-chassis-windows` requires a WGPU-compatible DX12 integrated or discrete GPU.
It does not silently fall back to WARP. Native composition uses GPU copies into
`CompositionDrawingSurface`; presentation never reads the rendered pixels back to
the CPU. The runtime's native mask paths drive both compositor alpha masks and
input geometry. TextBox editing remains in the real control.

Fractional native-mask color fidelity is not qualified. GPU content currently
sits behind the alpha-masked native visual; this arrangement can attenuate
partially covering foreground content twice. The same arrangement produced a
measured color mismatch on Linux. Correct foreground-layer separation is a
shared compositor requirement; opaque masking and input ownership checks do not
establish fractional blend fidelity.

## Lifetime and resource bounds

Three export batches retain their images until the native copy fence completes.
Each batch contains at most 64 images. The export budget defaults to 256 MiB and
can be set with `PAX_WINDOWS_EXPORT_BUDGET_BYTES` before creating the engine. It
counts DX12 resource allocation requirements across cached and outstanding export
generations, rather than only image pixel bytes. It is not a process RSS or
compositor memory limit.

A frame larger than this budget returns an error. Temporarily occupied capacity
applies backpressure while native input and runtime ticks continue. Runtime
native-control and mask patches wait with the pending frame; visible tile layout
changes only when the complete surface registry is ready to publish. The ordered
patch backlog is bounded at 65,536 messages and 8 MiB of serialized UTF-16 data;
exceeding either limit reports a terminal failure instead of growing indefinitely.
Resizing does not release a consumer-owned image. The native host separately caps its active
compositor tiles at 64 images and 256 MiB of pixel storage; outstanding copies
retain their old destination surfaces until their fences complete.

While delivery is blocked, runtime layout may advance beyond that visible scene.
Native controls dispatch their stable element IDs, but generic scene-pointer hit
testing can observe newer runtime geometry. Pointer/gesture fidelity during
prolonged backpressure is therefore not qualified by the native-control tests.

Close stops new rendering and polls asynchronously until application work, image
decodes, export tokens, and native copies retire. Device loss or another terminal
failure preserves unfinished ownership until process teardown. That path is
reported as a failure, never as clean retirement. Loaded cartridge code remains
process-pinned because Rust/driver callbacks and thread-local teardown can outlive
an individual resource session.

Dropping a prepared application before `run` is a separate boundary: no WinUI
dispatcher loop exists yet. The host pumps the original STA's messages and
cleanup wakeups without creating a window, waiting for at most the 15-second
cleanup deadline. A timeout or quit request reports failure and retains unfinished
ownership. Normal window close remains asynchronous. The smoke fixture's
`--prepare-drop` control checks repeated preparation and disposal without a GPU;
`--prepare-drop-async` additionally requires delayed worker acknowledgement.

## Assets and controls

The initial application path implements Text, Textbox, Button, Checkbox, Frame,
Scroller, and EventBlocker. Rich/editable Markdown text and additional native
control families return explicit unsupported errors. This is not a claim of full
native-target parity, including touch, IME qualification, pointer capture,
accessibility navigation, or every styling property.

Local images decode through WIC into straight RGBA for the runtime's image API.
This asset upload is separate from presentation. Local fonts resolve against
ordered `assets` roots and the corresponding `fontAssets` roots staged beside the
executable. A font with an empty URL selects a system family. Remote font CSS and
remote images need an explicit download policy and are not supported here.

## Building and qualification

`build.ps1 -Architecture x64 -Configuration Debug -OutputDirectory <absolute>`
builds `PaxHost.dll` and its runtime dependencies using MSVC v143 and pinned NuGet
packages. ARM64 and Release are also available. A compatible Windows App Runtime
must be installed. Preserve the complete output directory, including bootstrap
runtime dependencies. Cargo/CLI integrations build a scratch copy of authored
sources so NuGet caches do not become embedded SDK input.

The dynamically loaded host initializes the Windows App Runtime explicitly during
configuration, after the DLL loader lock is released. Bootstrap failures return
through the host error API. The runtime dependency remains valid for the
process-owned WinUI host's lifetime; DLL static initialization and teardown do not
perform bootstrap or COM shutdown.

Set `PAX_HOST_SDK_DIR` to the built SDK directory to use it when building the
`native-desktop-smoke` fixture. Direct Cargo loads the SDK from that directory;
packaged CLI output places the SDK beside the launcher. These are distinct launch
gates, especially for resource resolution.

In a matching MSVC shell, run the ownership tests on a DX12 hardware machine:

```powershell
$env:CARGO_BUILD_JOBS = '2'
$env:PAX_WINDOWS_GPU_TESTS = '1'
cargo test -p pax-chassis-windows --lib -- --nocapture --test-threads=1
```

Without the opt-in environment variable, these hardware tests report that they
were not qualified. They exercise delayed consumer fences, slot and byte
backpressure, resize generations, stale/duplicate releases, and close while
copies are pending. Repeat with `--release` for the release gate.

The ordinary application checker is
`tests/src/native-platform-probes/windows/check-native-app.ps1`. It requires the
built fixture executable, the native COM helper built by `build-uia.ps1`, and an
output directory. Run it in an interactive desktop session. It checks user-main
startup, native controls and Rust bindings, selection, nested scrolling, resize,
composed image pixels, and clean export retirement. It records explicit
exclusions instead of treating keyboard injection as IME qualification or a
requested font-family property as proof of the resolved font face. Build the
SDK-only helper with `build-font-metrics.ps1` and pass its executable through
`-FontMetricsExecutable` to compare the label's native text-range width against
DirectWrite shaping from the exact staged Manrope file. Distinct Arial and
Segoe UI measurements provide negative controls; an ambiguous staged font path
requires explicit `-StagedFontFile`. This additional gate must pass before
claiming bundled-font resolution. The older
isolated composition checker and its strict managed-UIA default are unchanged.

Set `PAX_WINDOWS_HOST_LOG` to an owned file for GPU and lifetime diagnostics.
Hardware/UI qualification is still required for changes to this host; a source
build alone establishes neither native input behavior nor composed-pixel fidelity.
