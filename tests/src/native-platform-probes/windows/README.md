# Windows native composition qualification

This standalone C++/WinRT probe tests the proposed native surface boundary. It
does not add a Windows target to the Pax CLI. Its optional `pax` producer connects
the real Pax renderer to the native composition boundary.

The [EC2 GPU lab provisioning runbook](../../../../scripts/native-gpu-lab/README.md)
defines a pinned NVIDIA Windows Server 2022 AMI pilot and an optional custom
image recipe. The authorized October 15 pilot launched the vendor image and
passed actual Pax/WGPU hardware export, offscreen rendering, and all 37 native
interactive checks on a Tesla T4 using explicit native COM UIA readback. This establishes
the bounded probe on Server 2022; it does not establish Windows 10 client support
or release performance. Packer image creation remains unexercised.

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
# Explicit selection-endpoint diagnostic; does not qualify UIA GetText:
.\check-interactive.ps1 -Architecture x64 -Producer pax -SelectionReadback Endpoints
# Explicit native COM UIA text-readback and selection checks:
.\build-uia.ps1 -Architecture x64
.\check-interactive.ps1 -Architecture x64 -Producer pax -SelectionReadback Native
```

Pass `-MSBuild` to `build.ps1` for an installation outside `C:\BuildTools`. Both
scripts also accept `x64` and `Release`. Run the interactive checker in an
existing logged-in desktop session; it refuses Windows session 0. It uses native
UI Automation and keyboard input, and writes `interactive-result.json` plus PNG
artifacts under `out`. Use `-KeepOpen` to leave the app available for manual work.

## Bounded hardware capability preflight

Run this before attempting interactive composition. It needs a matching Rust
MSVC toolchain and the current repository sources, but no WinUI window or logged-in
desktop. Build once from the matching MSVC developer shell:

```powershell
.\build-wgpu.ps1 -Architecture ARM64 -Configuration Debug
.\check-capabilities.ps1 -Architecture ARM64 -Configuration Debug
# Explicit software diagnostic; this cannot pass the hardware gate:
.\check-capabilities.ps1 -Architecture ARM64 -Configuration Debug -AllowSoftwareDiagnostic
```

The runner supervises `pax-wgpu-preflight.exe` with a 60-second process deadline
(`-TimeoutSeconds` changes it). Each invocation writes a new directory below
`out/capabilities`, containing `preflight.json`, streamed `events.jsonl`, stderr,
and the producer log. The report includes OS/architecture, executable SHA-256,
process exit status, WGPU adapter identity/driver, and DX12 HAL rejections that
occur before WGPU exposes an adapter. Timeout, crash, malformed/missing final
result, and renderer failures remain failures; existing evidence is never reused.

The hardware gate accepts only WGPU `IntegratedGpu` or `DiscreteGpu` candidates
that create the actual Pax context, render the retained vector scene and mask,
submit GPU copies to their DX12 exports, complete the queue, and destroy the
context. Every eligible candidate is attempted; one rejected device does not hide
another usable GPU. `Cpu` adapters are excluded by default. `VirtualGpu` and
`Other` classifications are reported separately and never qualify this gate.

With `-AllowSoftwareDiagnostic`, a `Cpu` adapter may additionally run the same
checks. A software-only success exits zero with `status=software-diagnostic-only`
and `softwareDiagnosticPassed=true`; **`hardwarePassed` stays false**. Without
that explicit switch, software-only availability exits nonzero. Automation that
requires hardware must check `hardwarePassed`, not only the process exit code.
The console executable also runs independently of PowerShell:

```text
pax-wgpu-preflight.exe --report new-preflight.json
```

It streams JSON events to stdout and optionally writes the final result to
`--report <new-path>`. The report path must not already exist; it is reserved
before driver initialization, so a crash leaves an empty or incomplete report.
Keep stdout for HAL rejection telemetry and require exit code zero plus
`hardwarePassed=true` for a hardware capability pass. The same explicit
`--allow-software-diagnostic` flag applies. Use the PowerShell runner or another
process supervisor to enforce the whole-process deadline, including when running
through a translation environment such as CrossOver. Translated GPU capability
does not qualify native Windows composition or physical Windows performance.

This preflight establishes Pax/WGPU device and export capability only. It does
not test WinUI composition or input, which still requires `check-interactive.ps1`.
Even a VM driver classified as integrated/discrete does not establish physical
GPU performance: both `nativeCompositionQualified` and
`physicalPerformanceQualified` remain false in preflight evidence. Physical
hardware, driver, composition correctness, and sustained performance must be
qualified separately before promoting a native host.

### Optional native interop API check

After the actual Pax/WGPU check, this small SDK-only console program measures
the next API boundary without installing WinUI. From the matching MSVC developer
shell:

```powershell
.\build-interop.ps1 -Architecture x64
.\out\x64\interop\pax-interop-capabilities.exe
```

Capture its JSONL stdout and enforce an external whole-process deadline, including
under CrossOver. It selects the first non-software DXGI adapter that creates a
DX12 device, creates D3D11On12 on its command queue, wraps a small BGRA texture,
reacquires and clears it through D3D11, releases it, and flushes. A DX12 fence
has a five-second wait before resource destruction. If that wait fails, the
process terminates without explicitly releasing potentially in-flight resources.
The `dx12Features` event records the selected native device's resource binding
tier; this diagnostic does not apply WGPU's separate adapter eligibility rules.

Before the GPU checks, the program independently attempts
[`DCompositionCreateDevice2`](https://learn.microsoft.com/en-us/windows/win32/api/dcomp/nf-dcomp-dcompositioncreatedevice2)
with a null rendering device, recording
`directCompositionWithoutRenderingDeviceCreated`. This documented factory-only
path cannot directly create surfaces. If D3D11On12 succeeds, it also attempts
activation with the same DXGI device, recording `directCompositionDeviceCreated`.
Both paths log HRESULTs, including missing DLLs or exports. Exit zero requires
both `d3d11On12ResourcePassed` and `directCompositionDeviceCreated`. Device
activation does not test a visual tree, compositor presentation, WinUI, or input.
It does not use Pax rendering itself; `actualPaxTested`,
`nativeCompositionQualified`, and `physicalPerformanceQualified` remain false.
Correlate this evidence with the separate actual Pax/WGPU preflight, without
treating successful API activation as native host qualification.
Run DirectComposition activation checks in a logged-in Windows desktop session.
The Windows ARM64 VM passed both activation paths and the D3D11On12 resource
check there, reporting binding tier 1; SYSTEM/session 0 returned `E_ACCESSDENIED`
for DirectComposition. A session-access failure does not establish missing API
support.

All interactive producers require a hardware device by default. `-AllowWarp` explicitly
permits software adapters or fallback for API qualification; it cannot qualify hardware
performance. Direct invocation accepts `--dx12`, `--pax`, and `--allow-warp`.

The checker verifies native UIA editor identity, real mouse ownership in the
known clear and visible mask regions, GPU frame publication and frame advancement
during selection, keyboard editing, horizontal and vertical scrolling, and text
retention through resize. It also requires a zero process exit code, a normal
XAML loop return, and (for `pax`) Rust context destruction after the native copy
fence. `-KeepOpen` leaves lifecycle qualification incomplete. Window placement
fits the monitor working area; pointer checks record and validate foreground
ownership, cursor position, UIA visibility, and client/scroll viewport bounds.
The clear-region point uses its UIA bounds when available, otherwise the report
identifies the fallback geometry tied to this probe's current affine transform.

`-SelectionReadback GetText` is the default. It reads the selected text through
the managed `TextPatternRange.GetText(-1)` API and checks the exact expected value. The explicit
`-SelectionReadback Endpoints` diagnostic first checks a collapsed caret at the
document start after Ctrl+Home, then verifies that Ctrl+A produces a nonempty
selection spanning `DocumentRange` before and after animated repaint. It uses
fresh selection ranges and strict endpoint comparisons; unexpected document-end
behavior fails instead of being normalized. The existing `ValuePattern` check
still verifies the editor's text. Endpoint mode never calls `GetText`, never
activates automatically after a failure, and reports that UIA text-range readback
was **not exercised and is not qualified**.

`-SelectionReadback Native` instead invokes the separately built, read-only
`native-uia.exe` helper through the Windows SDK's native COM UIA interfaces.
The default helper path is `out/<Architecture>/uia/native-uia.exe`;
`-NativeUiaExecutable` can select an explicitly built helper elsewhere. Each
call runs under a 20-second deadline, with fresh JSONL/stderr files beneath
`out/<Architecture>/<Configuration>/uia-results/<run-id>/`. The checker requires
zero exit, a complete set of successful API stages, matching window/process
identity, exact editor/document text, and exactly one selection. The optional
`IUIAutomation2` query may return `E_NOINTERFACE`; other API failures fail the
check. Ctrl+Home must produce empty selected text and a caret at the document
start; Ctrl+A must produce the full expected selected text and matching document
endpoints before and after animated repaint. The report qualifies native
`GetText` separately and explicitly leaves managed `GetText` unexercised and
unqualified. No mode is selected automatically after another mode fails.
Preserve managed failures separately; a native-mode pass does not establish that
the managed wrapper works.

`interactive-result.json` records the selected mode, run identity, and checkpoints
before UIA calls. A native checker crash leaves qualification incomplete rather
than retaining an older result. `selected-before-uia.png` captures the selection
before the default text-range readback. Screenshots crop the actual window using
physical pixel bounds. Inspect them for live masked text, selection, translucency,
tile continuity, and clipping; successful UIA assertions do not establish final
composed pixel fidelity. The app writes adapter information and rendering errors
to `composition-probe.log` beside the executable.

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

The latest Windows ARM64 WARP run passed all 14 interactive checker assertions,
including a clean exit, after the producer constructor was shared with the
preflight. This remains software-backed native composition evidence.

### EC2 Server 2022 hardware pilot

The October 15 NVIDIA Marketplace pilot uses x64 Windows Server 2022
`10.0.20348.5386`, Tesla T4 WDDM driver `596.86`, and Windows App Runtime
`1.8.260804001`. Actual Pax/WGPU hardware preflight, the offscreen alpha/clip/
resize test, and native D3D11On12/DirectComposition API checks pass. The desktop
runs through DCV over a private SSM TCP tunnel with runtime ingress closed.
Set an explicit desktop layout before input checks; the initial DCV display
state did not route the first mouse probe correctly. The corrected 1920×1080
run verifies foreground ownership, actual cursor position, and visible bounds.

Windows PowerShell 5.1's managed `TextPatternRange.GetText(-1)` and the finite
`GetText(4096)` control crash in `UIAutomationCore.dll` `7.2.20348.3692` on this
image. Both STA and MTA reproduce it. Managed selection endpoint queries also
return inconsistent ranges. The native COM client reads the same editor's exact
selected text and consistent endpoints successfully; keep these distinct API
paths in the evidence. A [similar upstream report](https://github.com/microsoft/Microsoft-UI-UIAutomation/issues/100)
exists, but it does not establish this image's root cause or a confirmed fix.
Use the explicit `Native` selection mode for this image while preserving the
managed-path failure. The final run passed all 37 checks, including exact native
text/selection readback, the collapsed-caret negative control, input ownership,
scrolling, resize retention, continued frame publication, and clean GPU teardown.
Inspection of its selected and resized captures confirmed live masked selection,
translucent Pax content, and clipping. These are bounded Debug correctness checks.
None of these checks substitutes for a screen-reader or
IME qualification pass, Windows 10 client coverage, or release performance.

### CrossOver renderer development checks

CrossOver can run x64 Pax/WGPU renderer checks through DX12 translation on the
Apple GPU. The latest offscreen pixel test passed on CrossOver 26.3. The setup
uses an isolated
`win11_64` bottle named `PAX1038-GPU-Probe`, with D3DMetal selected
(`CX_GRAPHICS_BACKEND=d3dmetal`). This records the bottle configuration, not proof
of every run's loaded translation backend. These checks are useful for renderer
development; they do not qualify a native Windows host.

Build the console executable and the existing offscreen test from a Windows
x64 MSVC developer shell. From the repository root:

```powershell
.\tests\src\native-platform-probes\windows\build-wgpu.ps1 -Architecture x64
cargo test --locked --manifest-path tests/src/native-platform-probes/windows/wgpu-export/Cargo.toml `
    --target x86_64-pc-windows-msvc --test offscreen --no-run -j 2
```

Stage `pax-wgpu-preflight.exe` and the exact test executable printed by Cargo
(renamed to `offscreen-test.exe` below) in a scratch directory outside the
repository, such as `/tmp/pax-1038-transfer`. Place the **x64**
`d3dcompiler_47.dll` from the locally installed Microsoft Windows SDK beside
both executables. CrossOver's built-in FXC could not compile the required shader
model 5.1 programs in this setup; the explicit native compiler override below
was required. Keep this SDK file out of the repository and its artifacts for
distribution; these instructions use a local SDK installation.

On macOS, use a fresh working directory for each run and capture stdout, stderr,
and process exit status. CrossOver's `--cx-app` requires a Windows path; passing
the macOS executable path to that option failed in the verified setup. The
`Z:` mapping below addresses the staged macOS files:

```sh
wine='/Applications/CrossOver.app/Contents/SharedSupport/CrossOver/bin/wine'
run_directory=$(mktemp -d /tmp/pax-1038-transfer/run.XXXXXX)
WGPU_BACKEND=dx12 "$wine" --bottle PAX1038-GPU-Probe --no-gui \
    --dll d3dcompiler_47=n --workdir "$run_directory" \
    --cx-app 'Z:\tmp\pax-1038-transfer\pax-wgpu-preflight.exe' --report preflight.json
WGPU_BACKEND=dx12 "$wine" --bottle PAX1038-GPU-Probe --no-gui \
    --dll d3dcompiler_47=n --workdir "$run_directory" \
    --cx-app 'Z:\tmp\pax-1038-transfer\offscreen-test.exe' \
    offscreen_preserves_alpha_clips_clean_frames_and_resize_lifetimes \
    --ignored --exact --nocapture
```

Use an external process supervisor with a whole-process deadline for these
commands. A GPU poll timeout does not bound driver initialization. The offscreen
test is ignored by default, so require `1 passed; 0 failed; 0 ignored` in its
result. Both the preflight and the pixel test rerun passed on CrossOver 26.3,
with one pixel test executed and none ignored.
The test checks RGBA/BGRA channel order, premultiplied alpha, clipping, retained
clean frames, resize, and output lifetime after renderer destruction. Unlike
the composition producer, this pixel assertion test intentionally reads pixels
back to the CPU.

The translated preflight and the 26.3 offscreen test reported
`AMD Compatibility Mode`, vendor `4098`, device `26287`, and `DiscreteGpu`.
Earlier 25.0 pixel-test traces identified Apple and MoltenVK, so those results
must not be attributed to D3DMetal solely from the bottle setting. Record the
CrossOver version, bottle configuration, adapter, and loaded translation
libraries alongside the evidence:
`hardwarePassed=true` and a driver's hardware classification here describe
translated GPU capability, not native Windows GPU qualification.

The separate native interop probe reported resource binding tier 3. On CrossOver
25.0, `D3D11On12CreateDevice` was missing (`0x8007007F`). On 26.3, the export
resolves, but device creation returns `DXGI_ERROR_UNSUPPORTED` (`0x887A0004`).
`DCompositionCreateDevice2(nullptr)` still returns `E_NOTIMPL` (`0x80004001`).
Therefore this configuration supports renderer testing only. WinUI, native compositor presentation/input,
and physical Windows performance remain unqualified; the preflight's
`nativeCompositionQualified` and `physicalPerformanceQualified` stay false.

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
