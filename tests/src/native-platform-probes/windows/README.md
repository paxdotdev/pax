# Windows native composition qualification

This standalone C++/WinRT probe tests the proposed native surface boundary. It
does not add a Windows target to the Pax CLI. Its optional `pax` producer connects
the real Pax renderer to the native composition boundary.

The scene keeps a real WinUI `TextBox` inside nested native `ScrollViewer`
controls. Two transparent `CompositionDrawingSurface` tiles overlay it. A
`CompositionVisualSurface` redirects the editor's live visual through an animated
alpha mask, with the original editor retaining input, selection, and accessibility
identity. The editor has an affine transform, and the scene has a rounded clip.
No `SwapChainPanel` or CPU pixel readback is used. A transparent input rectangle
covers the mask's permanently clear left strip, under the same affine parent.
This explicitly routes that region to the Pax side; the composition mask itself
does not change native hit testing. Arbitrary dynamic coverage and pointer
capture remain outside this bounded probe.

## Build and run

Use Windows with the Visual Studio 2022 C++ build tools, Windows SDK, and Windows
App Runtime 1.8 installed. `packages.config` pins the SDK dependencies;
`build.ps1` downloads NuGet 6.14.0 into the probe's ignored `tools` directory.

```powershell
.\build.ps1 -Architecture ARM64 -Configuration Debug
.\check-interactive.ps1 -Architecture ARM64 -Configuration Debug
# Explicit DX12 producer -> D3D11On12 GPU copy -> CompositionDrawingSurface:
.\check-interactive.ps1 -Architecture ARM64 -Configuration Debug -Producer dx12
# From a matching MSVC developer shell, build the isolated Rust producer:
.\build-wgpu.ps1 -Architecture ARM64 -Configuration Debug
.\check-interactive.ps1 -Architecture ARM64 -Configuration Debug -Producer pax
```

Pass `-MSBuild` to `build.ps1` for an installation outside `C:\BuildTools`. Both
scripts also accept `x64` and `Release`. Run the interactive checker in an
existing logged-in desktop session; it refuses Windows session 0. It uses native
UI Automation and keyboard input, and writes `interactive-result.json` plus PNG
artifacts under `out`. Use `-KeepOpen` to leave the app available for manual work.

All producers require a hardware device by default. `-AllowWarp` explicitly
permits software adapters or fallback for API qualification; it cannot qualify hardware
performance. Direct invocation accepts `--dx12`, `--pax`, and `--allow-warp`.

The checker verifies native UIA editor identity, real mouse ownership in the
known clear and visible mask regions, GPU frame publication, keyboard editing, selection
during animated repaint, horizontal and vertical scrolling, and text retention
through resize, a zero process exit code after closing, a normal XAML loop return,
and (for `pax`) explicit Rust context destruction after the native copy fence. `-KeepOpen` explicitly
leaves lifecycle qualification incomplete. Screenshots crop the actual window using physical pixel bounds. Inspect the images for live masked text, selection, translucency,
tile continuity, and clipping; successful UIA assertions alone do not establish
visual correctness. The app writes adapter information and rendering errors to
`composition-probe.log` beside the executable.

## Pax producer boundary

The isolated Rust crate depends on the sibling repository crates `pax-gpu`,
`pax-runtime-api`, and `pax-message`. Preserve those repository-relative paths
when staging it elsewhere. `build-wgpu.ps1` honors `CARGO_TARGET_DIR`, uses a
locked dependency graph, and copies the resulting DLL beside the native probe.
The library loads only from that adjacent path. It receives an explicit Windows
loader process-lifetime pin: Rust TLS destructors or deferred callbacks can
outlive a GPU context, so the probe does not unload their code on a still-live UI
thread. Normal load references are balanced and GPU contexts are destroyed; the
OS loader releases pinned code at process teardown. A production host can instead
statically link the Rust engine. Early dynamic unloading requires a separate
proven callback/TLS shutdown protocol.

Pax renders a retained transparent vector scene and a native coverage mask into
BGRA unorm offscreen targets. The same WGPU queue copies both into dedicated
export textures, then `transition_resources` records `COPY_SRC` in WGPU's tracker.
C++ wraps these textures through D3D11On12 using that same device and queue,
acquiring and releasing in `COPY_SOURCE`. It copies the scene into two compositor
surfaces and the mask into another. There is no CPU pixel readback and no direct
compositor access to Pax's mutable retained backing.

A bounded native fence wait precedes each new Pax frame and resource destruction.
This deliberately proves ownership with one frame in flight; it is not a
production scheduling design. `pax_wgpu` log entries identify adapter, renderer
frames, validation errors, and context destruction. DX12 HAL telemetry also
records adapters rejected before WGPU exposes them (for example, a resource
binding tier below WGPU's tier-2 requirement); successful native D3D12 device
creation alone does not establish WGPU compatibility.

Before closing, the app moves focus away from the editor in `AppWindow.Closing`,
while XAML input services still exist. This follows the app-side mitigation for
[WinUI's focused-TextBox shutdown defect](https://github.com/microsoft/microsoft-ui-xaml/issues/11653)
in the pinned runtime family. The checker leaves the editor focused so that
clean exit tests this app behavior.

## Observed qualification boundary

In the existing Windows ARM64 VM, native D3D11 and native DX12 producers render
on `Parallels Display Adapter (WDDM)`. WGPU 28's DX12 telemetry rejects that driver
(20.18.2640.57513) because its resource binding tier is below 2. The actual Pax
producer therefore requires explicit `-AllowWarp` there and logs Microsoft Basic
Render Driver / CPU. This tests the rendering and native composition API contract,
not hardware performance. Do not relax WGPU's adapter requirement or interpret a
successful native `D3D12CreateDevice` call as Pax hardware qualification.

## Remaining qualification

- Test real IME composition, candidate-window placement, clipboard operations,
  pointer selection, and scrolling gestures manually. The log records IME
  composition start/change/end; ordinary injected text is not an IME test.
- Verify hardware GPU operation separately from WARP and VM behavior.
- Qualify the optional Pax/wgpu producer on each adapter and in release builds.
  The default producer uses Direct2D; `dx12` uses native clear operations; `pax`
  uses the actual retained renderer. They share the same native control scene.
- Generalize the fixed-strip pointer ownership to arbitrary coverage, and test
  pointer capture, popup behavior, DPI changes, device
  loss, asynchronous frame lifetime, and sustained performance before promoting
  this probe into a native host.

The visual-redirection pattern follows Microsoft's
[WinUI Gallery OpacityMaskView](https://github.com/microsoft/WinUI-Gallery/blob/main/WinUIGallery/Controls/OpacityMaskView.xaml.cs).
The texture boundary follows
[Composition native interoperation](https://learn.microsoft.com/en-us/windows/apps/develop/composition/composition-native-interop).
