# Windows native composition qualification

This directory includes both the original isolated probe and acceptance runners
for the generated Pax Windows application. For the implemented host, its current
limits, and the `check-native-*.ps1` runners, start with the
[Windows host guide](../../../../pax-host/native/windows/README.md).

The standalone C++/WinRT probe below tests the native surface boundary. It
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

PowerShell scripts containing Unicode UI labels must retain a UTF-8 BOM for
Windows PowerShell 5.1. In particular, `website-acceptance.ps1` intentionally
includes this BOM; preserve it when editing. A syntax-only parser check can pass
while literals are decoded as ANSI and silently cease to match native text.
When embedding a script in SSM's `commands` string, decode the file with
`utf-8-sig` so the transport does not turn its BOM into executable command text.
This does not change the encoding of the staged script file.

For a top-level JSON array in Windows PowerShell 5.1, assign the conversion
directly: `$rows = [IO.File]::ReadAllText($path) | ConvertFrom-Json`. Wrapping the
whole conversion in `@(...)` can retain the parsed array as one nested element,
so `$rows.Count` reports one instead of the number of records. Validate the
expected record count and row shape before processing source or evidence
manifests; do not relax those checks to accommodate a nested array.

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

## Native wheel control contract

`build-wheel-child-contract.ps1` builds `wheel-child-contract.cpp` against a
specified native host SDK without modifying that SDK. Run the resulting
`WheelChildContract.exe` in an unlocked interactive desktop with
`PAX_WHEEL_CHILD_LOG` set to a fresh JSONL path. Its 28 cases check genuine
Slider, ComboBox, multiline TextBox, nested ScrollViewer, empty viewport, and
native scrollbar routes, including captured visuals. It requires exactly one
raw callback, window-space coordinates, preserved native defaults, cancellation,
and actual scrollbar hit ancestry; it never substitutes manual scrolling.

The October 25 Server 2022/T4 v5 host passed the focused eight-case ordinary
TextBox content/own-scrollbar test (141 assertions): native scrolling moved 48
units when allowed and stayed at zero when cancelled, with and without visual
capture. The broader 28-case executable then passed all 477 assertions on an unchanged-binary
retry after reconnecting the private DCV transport. It covers actual Viewer
scrollbars on both axes, truly empty viewports, TextBox content/own-scrollbar,
and Slider/ComboBox routes with visible/captured parity. Its earlier transport
failure after eight cases remains separate evidence; no assertion was relaxed.

The contract retains native Viewer template replacement coverage. It does not
reset TextBox's Template to null and clear it: that separate diagnostic collapsed
its native content extent and did not establish ordinary editor behavior.
Dynamic TextBox retemplating, open ComboBox popups, precision-touchpad raw events,
and real engine overlay ownership require their separate qualification gates.

## Packaged wheel callbacks

[`check-native-wheel.ps1`](check-native-wheel.ps1) exercises the genuine
[`native-wheel`](../../../../examples/fixtures/native-wheel) cartridge with real
`SendInput` wheel notches. Build the read-only observer in a matching MSVC shell:

```powershell
.\build-wheel-uia.ps1 -Architecture x64
.\check-native-wheel.ps1 `
  -Executable C:\fixture\native-wheel.exe `
  -OutputDirectory C:\evidence\native-wheel `
  -Profile Release `
  -ExpectedDpi 96 `
  -ExpectedExecutableSha256 <sha256-of-native-wheel.exe> `
  -ExpectedHostSha256 <sha256-of-PaxHost.dll> `
  -ExpectedCartridgeSha256 <sha256-of-native_wheel.dll> `
  -NativeUiaExecutable .\out\x64\uia-wheel\wheel-uia.exe `
  -ExpectedNativeUiaSha256 <sha256-of-wheel-uia.exe>
```

Use a fresh output directory, an unlocked interactive desktop, and enough room
for the fixture's 1024×780 logical client. The checker verifies foreground and
point ownership before input, loaded host/cartridge identities, and native scroll
positions against Rust-bound offsets. Each case requires exactly one inner and
one outer `ScrollViewer` peer with distinct Pax-owned automation IDs. Fourteen
cases cover exact once-only
handler paths, raw coordinates, both axes, event-time cancellation, Shift, rounded corners,
and a fixed sibling overlay over each nested viewport. The two vector-only
points must be outside observed native text bounds, with cancellation disabled
and both viewports stationary. It does not move offsets through UI Automation.

`wheel-uia.cpp` observes only the owned PID/HWND, ScrollPattern peers, and the
fixture's exact overlay label. Each child observer has a 20-second deadline and
flushed JSONL checkpoints; supervise the complete checker with a five-minute
external deadline too. Shared log reads retry only sharing violations and consume
complete snapshot lines; reports are published atomically. Readiness requires the
owned visible WinUI window and first frame, so a launcher's console window cannot
qualify. The terminal GPU close must report zero resources and equal published
and retired exports, followed by the successful drained marker. Final log errors
preserve the original failure, and the checker restores its environment and DPI
context. These additional harness gates do not change the 121 input assertions.
`check-wheel-pure.ps1` checks readiness, complete log lines, and owned-process
closure without opening a window or injecting input (18 checks). Supply
`-RetainedResult <result.json>` from the 121-check/14-case run to also validate
peer identity, event-time cancellation, negative mutations, and the report's
JSON roundtrip (58 checks total). Keep that retained input with the test receipt.
The observer establishes the measured fixture positions and bounds,
not general accessibility or managed UI Automation compatibility.

`-Profile` records the supplied build profile; retain its separate build receipt
to establish compiler settings. On October 26, fresh ordinary Debug (workspace
opt-level 1) and default baked Release (opt-level `z`, LTO enabled, one codegen
unit) each passed all 121 assertions across 14 cases, with 30 native observations,
96 DPI, normal exit 0, and exact terminal GPU retirement on Server 2022/Tesla T4,
WinUI 1.8. Both used the ordinary CLI with automatic SDK extraction/build and no
SDK override. The builds matched 784 audited inputs (778 relevant inputs and six
explicit exclusions) and 16 extracted SDK inputs; source and original lockfiles
were preserved. These packages include the GLES capability fallback, DX12 retained
buffer allocation charge, and opt-in allocator/startup diagnostics and subtimings.
This Windows run does not independently qualify the GLES backend.

The hardened checker and its helpers passed 124 native controls before these
runs. The canonical wheel-specific pure checker then passed its 58 checks with a
retained result; these overlap the 124 controls and are not additional distinct
cases. The first canonical wrapper attempt had a PowerShell inline-BOM transport
error despite its child checks passing; the separate BOM-free wrapper completed
successfully. The October 25 result below remains historical evidence from the
preserved v8 checker and its custom Release profile.

The October 25 run on Server 2022/Tesla T4, WinUI 1.8, and 96 DPI passed all 121
assertions across 14 cases, with 30 native observations and normal zero exit.
It used the refreshed CLI, 402 audited runtime/compiler/fixture inputs, a pinned
v5 SDK override, and baked Release with opt-level 3, LTO off, and 16 codegen units.
That profile matches the retained website dependency cache; it differs from the
workspace's default size/LTO profile, qualified by the October 26 runs above.
Precision touchpads, touchscreens, and open native popups remain separate scopes. The frozen package predates the
later GLES image-view fallback; that shared-renderer delta has separate tests.

| October 25 artifact | SHA256 |
| --- | --- |
| `pax-cli.exe` | `e05d761fdc9f4c522496f196d284397f2a61c2618bd5f4bbada3d5c48c851f94` |
| `native-wheel.exe` | `3845a87227d7150db18c5c770bf5e126c7d226b27489c283b4317e13e85d432b` |
| `native_wheel.dll` | `73d1e9c2653e63f8a5f880927aaa9d9d121f87c0106a9b1a44ef0290387f46ec` |
| `PaxHost.dll` | `3ab8c96b5a1a0e9f135d572d6bcac94e407cac332f7edccbb68ebded5f7a1d9d` |
| Checker source | `b48cb8eda1231a64db3221e0ae2877fdb2f919b8931e330e478d84ddbdee02f2` |
| Observer source | `1fe0d929fb343a62c383f1a067a8e047db32cca4cd748aa2018835bc6cda0b94` |
| Observer executable | `a8b6541d31771007469303aa3a781aed026254ecf742ac1a851a2d243d282579` |

The retained v3/v4 attempts failed in harness snapshot handling before any input
case; they do not count as passes. V5 and v7 were parsed but never run. The
retained v6 passed 118 checks; v8 adds positive event-time cancellation and
requires distinct inner/outer peers. All 121 v8 checks passed on the same immutable
package. Separate negative controls rejected duplicate peers and disabled or
false event-time cancellation receipts. Evidence includes
`acceptance-v8/result.json`, raw fixture snapshots, all native observer JSONL
checkpoints, screenshots, and the separate build/source/binary receipts.

## Native text measurement qualification

[`check-native-text-layout.ps1`](check-native-text-layout.ps1) checks the generated
[`native-text-layout`](../../../../examples/fixtures/native-text-layout) fixture
against both its Rust layout snapshots and genuine WinUI UI Automation peers.
Build that fixture separately in Debug and Release, and build the native
observation helper with `build-ime-uia.ps1`. Run the checker from an interactive
Windows desktop with a working area of at least 1040×1020 physical pixels:

```powershell
.\check-native-text-layout.ps1 `
  -Executable C:\fixture\native-text-layout.exe `
  -ImeUiaExecutable .\out\x64\uia\ime-uia.exe `
  -OutputDirectory .\out\text-layout `
  -ExpectedTextScale 1 -KeepAlive
```

The checker records actual system text scale and window DPI, engine/native
bounds and positions, accepted measurements, native text ranges, and screenshots.
The final nonblank character must have an in-bounds range with the same line
height as the fixture's matching single-line label. The fixed-height label at
200% provides a clipping negative control. These checks qualify vertical line
layout for this fixture; UIA rectangles do not establish pixel-level glyph ink
or general occlusion correctness. It exercises
implicit dimensions, constrained-width wrapping, content and width edits,
autosized Stacker/Frame parents, repeated text siblings, and an explicit
30-pixel-height control. It uses the fixture's opt-in `PAX_TEXT_LAYOUT_LOG` file;
no development inspection endpoint is required. Supervise each checker process
with a bounded external timeout, because a native UI Automation call can hang.

For a live text-size check, change the real Windows accessibility **text size**
setting, then attach using the previous result's `processId`, `appStderrPath`,
`hostLogPath`, and `engineLogPath` as `-AttachProcessId`, `-AppStderrPath`,
`-HostLogPath`, and `-EngineLogPath`. Pass that result file as
`-PreviousResultPath` and the new factor as `-ExpectedTextScale` (for example
`1.5`). The first snapshot runs before content/width edits or window resizing;
the comparison requires a successful prior run, matching process start time and
binary/helper identities, unchanged DPI and content state, plus changed
intrinsic geometry. `-SnapshotOnly` omits subsequent mutations. Cold-launch
checks require a fresh process at each scale. Restore the original system
setting afterward. Without `-KeepAlive`, normal close must retire all owned GPU
exports. Retained live processes must also be closed and their final host
counters checked explicitly.

On October 19, on Windows Server 2022 with a T4 GPU, the Debug and baked Release fixture passed
cold-launch checks at 100%, 150%, and 200% accessibility text size with display
DPI held at 96. The final packages also passed unchanged-process live changes
from 150% to 100% in Debug and from 200% to 150% in Release, including parent and
repeat propagation before content edits. Normal close retired all owned exports.
These results do not qualify Windows client editions, screen readers, mixed-font
line metrics, or arbitrary text transforms.

## Native controls and scrolling diagnostics

Build the [`native-controls`](../../../../examples/fixtures/native-controls/README.md)
fixture in both Debug and baked Release. In an interactive desktop session:

```powershell
.\check-native-controls.ps1 `
  -Executable C:\fixture\native-controls.exe `
  -OutputDirectory C:\evidence\native-controls `
  -ExpectRounded -ExpectOffGrid
```

The runner drives actual keyboard and pointer input and observes genuine WinUI
automation peers. It checks shared Slider values and event counts, off-grid
authored values, range changes, dynamic options, independent radio groups,
transformed popup selection, and conditional teardown. `-ExpectRounded` requires
standalone Scroller corner pixels and input exclusion to agree; without it the
radius behavior is recorded as a baseline observation, not a passing clipping
test. `-ExpectOffGrid` requires the fixture's Off-grid value button. Drag testing
uses delivered `SendInput` movement rather than merely warping the cursor with
`SetCursorPos`. Normal close must exit zero and report complete export retirement.

[`scene-clip-contract.cpp`](scene-clip-contract.cpp) separately checks the native
Scroller's `clip_content` true → false → true transitions and zero-size
restoration after ordinary layout updates. It also checks EventBlocker's actual
native Grid background: default transparency, opaque and translucent updates,
retention when the field is omitted, and explicit transparency restoration while
remaining hit-testable. Build it against an existing generated
host SDK; the wrapper reuses that SDK's pinned packages without downloading or
overwriting its production outputs:

```powershell
.\build-scene-clip-contract.ps1 `
  -SdkSourceDirectory C:\fixture\native-sdk `
  -OutputDirectory C:\evidence\clip-contract
$env:PAX_CLIP_CONTRACT_LOG = 'C:\evidence\clip-contract\result.jsonl'
& C:\evidence\clip-contract\SceneClipContract.exe
```

Run in an interactive desktop under an external 20-second process deadline.
Require six passing phase rows, five passing `blocker-` rows, passing `complete` and `application-returned`
rows, and process exit zero. This tests geometric-clip state and the absence of
rounded-only mask/capture allocations; the fixture runner above provides the
separate corner-pixel and input checks.

For a bounded run of `scroll-matrix`, `rounded-scroller-tiles`, or another fixture
with actual native Scrollers:

```powershell
.\check-native-scroll.ps1 `
  -Executable C:\fixture\scroll-matrix.exe `
  -OutputDirectory C:\evidence\scroll-matrix -Seconds 60 `
  -ExpectedFonts "Fraunces|Azeret Mono"
.\check-native-scroll.ps1 `
  -Executable C:\fixture\neon-opacity.exe `
  -OutputDirectory C:\evidence\neon-opacity -Seconds 60 `
  -ExpectedFonts "Oxanium|Space Mono" -AnimationOnly
```

This runner samples native wheel movement, CPU time, working set, private bytes,
handles, and threads. It enables `PAX_WINDOWS_TIMINGS=1` and records the host's
bounded callback/publication distributions. It requires at least one native
viewport to move, continued application life, zero exit status, and clean export
retirement; it is not a complete nested-axis or clipping oracle.
`-ExpectedFonts` checks the native text peers' reported family names.
`neon-opacity` has no Scroller and requires `-AnimationOnly`, which records
animation diagnostics without sending wheel input or claiming scrolling coverage.

Keep startup, input, capture, and shutdown events in the evidence. Host callback
work and successful publication are separate from GPU execution, actual display
presentation, and remote-desktop delivery. Export-pool counters do not include
all Composition mask, redirected-visual, or driver allocations. A short passing
run does not establish long-term memory stability or physical input latency.

## Native Text style and measurement

`build-text-style-contract.ps1` builds a real NativeScene/XAML contract probe.
It covers initial zero-size Text, explicit versus unconstrained measurement,
zero-to-positive and positive-to-zero transitions, retained content/native identity,
and generation-checked measurement recovery alongside the existing style checks.
The same contract also checks Scroller inner-pane dimensions through negative,
zero, positive, and negative updates while retaining the native scroller identity.

Set `PAX_TEXT_STYLE_CONTRACT_LAYOUT_ONLY=1` to run those layout/style assertions
without native pointer gestures. Its result explicitly records this restricted
scope; the default additionally requires foreground ownership and native taps.
Run `TextStyleContract.exe` with `PAX_TEXT_STYLE_CONTRACT_LOG` and an external
30-second deadline. Require all phases, `application-returned`, and exit zero.
It checks intrinsic content in fixed boxes, separate block/line alignment,
underline changes, implicit sizing, reactive native control labels, centered
unclipped overflow, and explicit authored-box clipping. Dropdown phases retain
the selected faceplate label through replacement, selection changes, empty/refill,
and selected-index invalidation without programmatic change echoes. Real native
taps test selectable Text toggles and exact Click counts.

Build `examples/fixtures/native-text-style` in Debug and baked Release and use
`check-native-text-style.ps1` for composed screenshots, native text attributes,
line rectangles, reactive styles, and genuine RadioList text. RadioButton labels
expose their accessibility name without a child TextPattern; their authored
foreground is checked against composed pixels within each native control's bounds.
Pass
`-ExpectedHostSha256` to verify the loaded host identity. The gate also checks
popup Escape/focus, keyboard selection, and editor text retention across a style
change. Textbox underline remains unsupported; Button/Dropdown/RadioList label
decorations are checked by the native contract.

The autosize fixture's fixed-height negative control explicitly sets `clip=true`.
Its checker compares implicit axes with native measurements and constrained axes
with the authored box; intrinsic text peers need not fill an explicit width.

## Navigation, default fonts, and a newly created project

Build `examples/fixtures/native-navigation` in Debug and baked Release, then run
`check-native-navigation.ps1` with expected executable, cartridge, and host hashes.
The gate compares the actual Router log with visible native route state, exercises
selectable Link drag/copy versus tap, and requires clean process/GPU retirement.
Pass `-FontMetricsExecutable` to compare all three Roboto faces with the exact
staged file and discriminating fallback metrics. The native helper selects the
ASCII diagnostic suffix from the document end, avoiding assumptions that UIA
Character units equal UTF-16 offsets across the preceding text.

`-SelectionDiagnostic` runs `inspect-native-selection.ps1` in a separate bounded
MTA client. On the qualified Server 2022 image, managed selected-range `GetText(-1)`
crashed in UIAutomation in both STA and MTA, consistent with the older editor
pilot recorded below; native Copy of selected Link text succeeds. The optional
`selection-uia.exe` helper uses native COM in a bounded MTA process for a separate
TextBlock-specific comparison. Both finite `4096` and unbounded `-1` native COM
reads returned the exact selected Link text in the October 21 run, while the
managed client still crashed. Keep the managed diagnostic failure separate from
the passing native API and functional selection gates.

`check-native-created-logo.ps1` exercises the unmodified default project generated
by `pax-cli create`: genuine Slider input, Replay, reactive background, and clean
close. Pin all three loaded identities. `-WorkingDirectory` supports a relocated
receiving package from an unrelated directory; record the receiving token,
prerequisites, and source/SDK access separately from the application's UI result.
The October 21 receiving run passed 25 checks under a fresh Users-only account
with source/compiler reads denied, using the untouched default CLI Release
package. Its existing machine App Runtime framework registered for that user
on first launch; a clean-machine prerequisite installer remains unqualified.

## Native snapping and Button states

Build `examples/fixtures/native-scroll-snap` in Debug and baked Release, then
run the native wheel, dynamic policy, and Button styling checks. Pin the expected
package identities so a copied but unused candidate cannot be mistaken for the
loaded application or SDK:

```powershell
.\check-native-snapping.ps1 `
  -Executable C:\fixture\native-scroll-snap.exe `
  -ExpectedHostSha256 <sha256-of-qualified-PaxHost.dll> `
  -ExpectedExecutableSha256 <sha256-of-native-scroll-snap.exe> `
  -ExpectedCartridgeSha256 <sha256-of-native_scroll_snap.dll> `
  -OutputDirectory C:\evidence\native-snapping
```

The checker records the package identities and actual loaded module paths and
hashes. Expected hashes are optional for existing callers; when provided, each
must match before behavioral results count. Record retained-cartridge and copied
host provenance explicitly when testing a host against an older package. Check
launcher/configuration export selection and cartridge ABI compatibility before
substituting a host: a retained config without `nativeExports` cannot use the
current host. A startup rejection from an incompatible package does not qualify
snapping behavior.

The gate requires a connected interactive desktop, a real WinUI window, and
96 DPI for its exact native Button color/outline samples. It checks normal,
hover, pressed, and reactive style changes; horizontal/vertical wheel snapping;
rapid wheel input; repeated off-anchor writes; dynamic clearing/restoration;
and percentage anchors after viewport resizing. Bound offsets are compared
with UI Automation's native scroll positions. Passing requires a responsive
window, graceful zero exit, and complete export retirement. Retirement requires
the final `closed_gpu` record to have zero exported bytes/images and pending
bytes/frames, `closing=true`, and equal published/retired counts, followed by a
terminal successful drained marker. An earlier zero-valued initialization
record cannot satisfy this gate. Atomic report publication and bounded
sharing-only live-log retries preserve complete evidence; a final-log read error
fails the run without replacing an earlier input or startup failure.

Before application input, run the pure helper controls with the same checker:

```powershell
.\check-snap-pure.ps1 -Runner .\check-native-snapping.ps1
.\check-retirement.ps1 -Runner .\check-native-snapping.ps1
.\check-snap-retirement-errors.ps1 -Runner .\check-native-snapping.ps1
```

These cover native peer identity/axis/offset predicates, terminal record
selection, and preservation of the original error if retirement-log reading
fails. This does not qualify touch/trackpad momentum or uninterrupted inertia.

`build-snap-point-contract.ps1` builds an isolated native provider contract
against the specified SDK source and cached packages. Run its
`SnapPointContract.exe` on an interactive desktop with `PAX_SNAP_CONTRACT_LOG`
set, impose an external timeout, and require both the final `complete` and
`application-returned` records plus process exit zero. The probe covers native
interface identity, immutable views, clamping/sorting/deduplication, axis
notifications, clearing/refilling, extent changes, invalid input, and stable
policy revisions for unchanged updates.

The experimental ScrollViewer adapter uses `ChangeView` and final
`ViewChanged(IsIntermediate=false)` notifications. Unlike ScrollView, it has no
request correlation ID. Accepted authored requests are serialized; a newer target
remains queued until the earlier final event, and an older deferred reply must
not acknowledge that newer target. A repeated off-anchor request can return
`false` with no movement event; the host must then acknowledge the actual snapped
position once the control has a valid, settled viewport. The native snap-motion
and integrated NativeScene scroll contracts pass on Server 2022/T4, covering
dynamic anchors, resize, clear/restore, no-motion cases, and overlapping authored
requests. On the morning of October 26, 2026, fresh packaged applications passed
all 36 checks in both ordinary Debug and workspace-default baked Release (`opt-level="z"`,
LTO, one codegen unit). Both used `pax-cli build` with automatic embedded SDK
extraction/build and no SDK override. The morning 782-input source audit matched
all 776 relevant inputs, including the shared-image capability fallback and Windows opt-in
allocator/startup diagnostics; 16 extracted SDK inputs matched their source.
The exact executable, cartridge, and loaded host identities were verified.
Both applications exited normally with zero remaining exported/pending resources:
Debug published/retired 85 frames; Release 84.
These morning packages predate the later DX12 vector-buffer budget correction
and additional engine startup subtimings. Their provenance remains separate from
the final-core reruns below.

The as-run behavioral checker was `27c9e504...`; Release used the stronger exact
terminal parser in its outer diagnostic gate, and the retained Debug log passed
the same parser separately. Canonical checker `4c5c3b4f...` incorporates that
parser and final-log failure preservation. In the morning packages, the allocator
report was absent with its flag off in Debug and supported with bounded labels
when enabled in Release. These initialization/close checks do not establish a
steady-state memory plateau.

Later that day, fresh final-core packages using canonical checker `4c5c3b4f...`
each passed all 36 checks in ordinary Debug and default baked Release. Their
784-input audit matched all 778 relevant inputs and 16 automatically extracted
SDK inputs, including the DX12 buffer-budget correction and engine subtimings.
No SDK or profile override was used: Debug retained engine opt-level 1 and fixture
opt-level 0; Release retained opt-level `z`, LTO, and one codegen unit. All 91
native helper controls passed before compilation. Exact package/loaded-module
checks, current T4/596.86 identity, unlocked 1920×1080 desktop and 96 DPI were
verified. Both applications exited normally with code 0 and exact terminal GPU
retirement; their owned tasks retired without outer cleanup. The original source,
lockfiles, morning packages and both website packages were preserved. Physical
touch/trackpad input and Windows client versions remain unqualified.

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
Those October 15 checks did not establish screen-reader or IME behavior,
Windows 10 client coverage, or release performance.

The October 18 follow-up passed all 62 assertions in the reusable
[`check-native-ime.ps1`](check-native-ime.ps1) with `-CheckGeometry`, using a fresh
generated Release `native-desktop-smoke` process on the same Server 2022/T4
configuration with the installed Microsoft Japanese IME. It observed real preedit
and conversion ranges, candidate UI Automation identity, commit/cancel,
selection, Rust binding updates, and isolation from canvas keyboard handlers.
Candidate placement followed window translation, a three-degree editor rotation,
and nested scrolling before composition. The checker also verified active
composition immediately before close, clean export retirement, and restoration
of the exact original English keyboard layout. Debug IME execution remains
unqualified. These results qualify that installed IME and Release fixture, not
other IMEs, client Windows 10/11, reconversion, screen-reader behavior, or
composition during ongoing transforms or scrolling.
See the [host's IME checker guidance](../../../../pax-host/native/windows/README.md#building-and-qualification)
for prerequisites, the native observation helper, and evidence capture.

The same follow-up passed 49 application checks and 61 keyboard checks at actual
125% display scaling (120 DPI), with `TextScaleFactor=1`, then restored the
original 96 DPI and text scale. This bounded result does not establish live or
multi-monitor DPI transitions. Separate 150% and 200% text-only scaling exposed
clipping in the fixture's fixed 30-pixel-high labels; display-scale acceptance
does not qualify text reflow or intrinsic layout adjustment.

The relocated Release package also passed all 49 application checks under a
fresh Users-only account, with a Windows-only PATH and no Pax SDK or Rust
environment overrides. Runtime-module observations confirmed Windows App Runtime
1.8 (`8000.946.1701.0`) from WindowsApps and VC++ `14.44.35211.0` from System32.
No additional runtime installation was needed for that user. This establishes
fresh-user operation on the prepared image, not clean-machine prerequisite
installation. The disposable account was disabled after qualification.

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

- Extend the bounded Microsoft Japanese IME results above to other IMEs,
  reconversion, and composition during ongoing transforms or scrolling. Qualify
  clipboard operations, pointer selection, and scrolling gestures separately.
  Ordinary injected or pasted Unicode text is not an IME composition test.
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

For bounded memory diagnosis, `check-native-scroll.ps1 -AnimationOnly -MemoryPhases`
with neon-opacity divides `-Seconds` into equal animated, paused, and resumed
periods using the real pause button. It records phase timestamps and five-second
process samples. Enable `PAX_GPU_MEMORY_DIAGNOSTICS=1` and
`PAX_PROPERTY_MEMORY_DIAGNOSTICS=1` before launching to pair these samples with
shared live allocation counts and bytes. `PAX_WINDOWS_TIMINGS=1` adds native
live composition counts plus cumulative transform/clip/brush/input-path creation
counts. Cumulative counts measure allocation activity, not retained memory;
logical texture bytes exclude driver overhead. Keep compiler work outside the
measurement window and distinguish memory retention from actual display rate.

`PAX_WINDOWS_PHASE_TIMINGS=1` separately enables cumulative and interval host
phase wall time plus current UI-thread CPU time: native-copy polling, decoded
image delivery, viewport preparation, runtime tick, native patch queueing, surface
planning, rendering, native apply, native commit, publication, and callback tail.
It samples thread clocks only when enabled and emits summaries at existing
120-publication checkpoints and shutdown. First and maximum wall durations are
cumulative; interval totals can be divided by interval counts. Thread CPU uses
Windows accounting granularity and excludes workers, GPU work, and WinUI work
outside the callback; interpret accumulated windows rather than individual tiny
phase CPU values. Phase wall durations include response decoding and any diagnostic
log bookkeeping between the stage marks. These queries and logging add overhead; compare
the exact same binaries with the option disabled before making performance claims.

### Website acceptance workload

The root `pax-website/` application is the desktop alpha robustness workload.
Build its unchanged source with `pax-cli build --path pax-website --target windows
--release`, then use `check-native-scroll.ps1` on the packaged executable. Run an
`-WebsitePhases -Seconds 30` observation for stationary animation, first traversal,
and repeat traversal. It uses real wheel input at the outside margin and checks
that the outer native scroller reaches both settled endpoints. CPU and memory
samples are grouped by phase; host timing logs retain their rolling-window scope.
The probe then resizes to a 720-pixel logical client width and restores the wide
viewport. Add `-OpenSource` to exercise the first source drawer after these phases;
the source drawer must expose selectable multiline native text. Add `-SourceOnly` with
`-WebsitePhases -OpenSource` to isolate the drawer when a separate scrolling
baseline is blocked; that mode skips the timed traversals and resize checks and
does not qualify them. The source probe uses real drag selection and Ctrl+A/C,
then compares clipboard text with the native document, including Unicode and
line breaks. Passing `-NativeSelectionExecutable` additionally checks the selected
range through the separate bounded native COM observer; that optional observer
is not part of the default run. The probe does not qualify every calculator,
nested demo, selection gesture, or visual-layout interaction.
Use `-AnimationOnly -PreserveStartupViewport -StartupSequence` for a separate default-size startup
and short idle observation. A native render error fails the probe even if its
window remains responsive and resources close cleanly. The general
`-AnimationOnly` and ordinary scrolling modes remain available for other fixtures. Pass `-ClientWidth 1280 -ClientHeight 960` for
a comparable logical client viewport; startup observations retain the application
default size before this resize, and the report records actual pixel dimensions. Keep source, executable, cartridge, and host
hashes with each result; report unresolved asset or native-control failures
before interpreting performance. The initial comparison should retain the
website's fonts, images, shaders, native scrollers, and source drawers.

For a separate bounded memory capture, use `-WebsitePhases
-WebsiteMeasurement memory -WebsiteDiagnosticsMode enabled`. This declares 90
seconds at the top, three real-wheel down/up round trips (each leg bounded by 90
seconds and required to reach its settled endpoint), then 90 seconds at the top.
`-WebsiteMeasurement stationary` declares one 45-second top interval for each
fresh-process diagnostics comparison. These modes exclude source interactions,
startup screenshots and the usual responsive resize sequence. Initial/final
screenshots stay outside measured intervals; animations continue throughout.
They do not establish a globally paused scene or complete website acceptance.

Explicit `-WebsiteDiagnosticsMode enabled` or `disabled` sets Windows, GPU memory,
render and property diagnostics to `1` or `0`, with Windows phase timings `0` in
both modes. The default `existing` mode retains inherited diagnostic settings and
the probe's usual Windows timings. Changed environment values are restored on
exit. Record the same package and viewport for every A/B launch, and pass
`-LaunchOrdinal` to retain the declared order. `websiteMeasurement` records the
run/process identity, exact flags, package hashes, viewport and fixed plan.
Before the first measured sample, the probe places the pointer at a checked
integer point near the visible outer viewport's 2% horizontal margin and vertical
center, then records the requested and actual screen coordinates. This makes
placement deterministic without asserting that hover or animation is disabled.
Website samples bracket counter and trace-offset collection with UTC milliseconds;
phase records reference their first/last sample IDs. Sampling stops if working set
exceeds 6 GiB or process private bytes exceed 8 GiB; the failed result preserves
earlier samples and does not qualify a complete measurement. Sampling, UIA observations
and report serialization still add overhead in either diagnostic mode. Use
`check-website-measurement.ps1` for pure plan, parameter and sample-bracket checks.

`-StartupSequence` captures native screenshots at the first available observation
and approximately 2, 5, and 10 seconds after first publication, before the UIA
inventory. Each image records its actual launch/publication-relative timestamp.
Review those pixels separately for settled text layout; the probe does not infer
readiness from the first frame or from changing animation pixels.


The probe records executable launch to the first visible WinUI window and first
published Pax frame separately from compilation. `firstWinUIWindowObservedMs`
requires the `WinUIDesktopWin32WindowClass` class and native visibility. The older
`firstWindowObservedMs` field is retained as an unclassified process-window
observation; a launcher window can precede WinUI. New reports also retain that
first window's class. Its 200 ms polling and host log observation provide startup
bounds, not display presentation timing. CPU,
private bytes, working set, and callback/publication intervals likewise describe
the guest application; DCV stream rate is not local display frame rate. The
initial and final screenshots occur outside the timed interaction window.
Actual source-drawer, nested demo, and responsive-layout interactions still need
functional checks in addition to this general scrolling probe.

`make-website-source-manifest.py REPOSITORY OUTPUT.json` freezes the canonical
website's 92 authored source-tab identities without copying their contents. It
recognizes literal pairs, language tuples, and `ExampleSource` entries, preserving
the six drawers' authored order and repeated source occurrences. It also writes
`OUTPUT.sections.json`, binding the authored section order, unique visible
section anchors, per-drawer inventory, and source provenance to the exact source
manifest hash. Stage both files together. The scenario rewinds the native page
once, then finds each next section through real wheel input; it never sorts empty
offscreen UIA bounds. Every repeated source label must qualify in its own drawer.
Pass
that file with `-WebsitePhases -WebsiteInteractionsOnly -WebsiteSourceManifest
OUTPUT.json` for the companion `website-acceptance.ps1` scenario: calculator
keypad/graph switching, Paxflix demo feedback and theme controls, gallery wheel
input, exact native source text for every tab, and the authored `/calculator/`
and Back to Pax route roundtrip. Route retirement allows authored exit timelines to finish
before requiring old native scrollers to disappear. This mode skips the separately
measured traversal/resize phases. Preserve each failed probe attempt and its
geometry diagnostics; a harness failure does not establish an application defect.
Source-tab clicks record fresh label and viewport bounds, the physical click
point, UIA hit ancestry, and scroll position. The current drawer has no separate
selected-file label; the exact document hash establishes selection. Source text
discovery stays inside the newly mounted native code scroller, identified by
its new native identity and its alignment immediately below the tab scroller.
An ambiguous or missing code viewport fails the probe. A failed
exact-text check preserves its assertion and screenshot before trying a bounded
interior click as a separate diagnostic. That later click cannot turn the original
failure into an acceptance pass or count the tab as qualified.
Physical click coordinates must remain inside the half-open visible rectangle.
The shared helper floors the center and clamps it between `ceil(left)` and
`ceil(right)-1`, with the same rule for Y. This preserves one-pixel clipped-tab
coverage; a subpixel slice containing no integer coordinate continues the normal
wheel search. Each source click independently asserts label and viewport
containment. Run `./check-click-point.ps1` for the coordinate-only regression:
odd/even one-pixel edges, fractional and negative origins, unclickable slices,
and negative controls for the original rounding onto an excluded edge. It parses
the common runner's helper without launching an application.
Progress reports are published by atomic replacement. Sharing violations retry
at most six attempts, 40 ms apart; a persistent blocker fails publication while
preserving the previous complete report. Progress readers must allow
`FileShare.ReadWrite | FileShare.Delete`; `Read-ProbeReport` implements that rule
with bounded retries. `./check-report-publication.ps1` checks held compatible and
incompatible readers, recovery, and concurrent reads without partial JSON. A
prior run's sharing failure does not identify which process held the file.
Live host-log reads separately use `Read-ProbeLog`, with the same sharing flags
and six bounded attempts only for sharing/lock violations. Other read errors
fail immediately; first-publication and retirement markers remain required.
`./check-log-sharing.ps1` checks held append writers, the original reader failure,
transient and persistent blockers, missing/access errors, and concurrent appends.
A failed retirement read is recorded without replacing an earlier run failure.
Retirement requires the final `closed_gpu` JSON record followed by a terminal
`closed exports_drained=true` marker. Its bytes, images, pending bytes and pending
frames must be zero, closing must be true, and published/retired counts must
match. Earlier initialization zeros cannot satisfy this check.
`./check-retirement.ps1` includes earlier-zero/later-nonzero, missing/malformed,
ordering and counter-type negative controls; `-RetainedLog` writes a separate
revalidation receipt without changing historical run results.
Add `-WebsiteSourcesOnly` to isolate the six drawers, all source tabs, and the
route roundtrip on a fresh launch. That restricted pass does not qualify the
calculator, Paxflix, or gallery interactions; retain their separate results.
Source drawer traversal requires a wide layout of at least 1000 logical pixels;
use `-ClientWidth 1280 -ClientHeight 960`. Below that breakpoint, OpenSource's
stacked layout moves its source-discovery anchor and needs a separate oracle.
Ordinary page targets are revealed below the website's fixed navigation.
General control clicks read fresh visible bounds immediately before input and
record their coordinates; authored animations need not stop before a click.
Clipped source-tab frontier clicks retain their separate exact containment checks.
The route reveal explicitly accepts the full visible viewport because a footer
at maximum scroll may never enter the default upper targeting band. It asserts
the final integer click is inside both visible label and viewport; source reveal
and frontier rules retain their existing behavior. `./check-route-point.ps1`
checks this pure geometry, including the bottom footer, clipped one-pixel edges,
unclickable fractional slices, and excluded edges. Its seven checks and native
parsing passed; the October 26 route-only application replay also passed below.
Add `-WebsiteRouteOnly` instead to replay only the authored calculator link,
old-scroller retirement, and Back to Pax wheel response on a
fresh launch. Its report scope is `route`; it does not qualify source drawers or
embedded demos, and it cannot be combined with either other restricted scope.
HomePage binds its scroll offset to the surviving root component, so returning
home need not start at zero. The wheel check chooses the direction with room to
scroll and requires movement in that direction after the new scroller mounts.
Conversely, `-WebsiteDemosOnly` isolates the embedded interactions, skipping both
source traversal and the route roundtrip. Graph-wheel
diagnostics compare the chart center with a blank interior point, record the
native hit ancestry and offset samples, and test authored dragging. Failed graph
checks remain failures while the remaining demo checks continue, so a hit-routing
problem does not conceal unrelated modal or gallery defects.
Keep the companion beside the main script, and stage PowerShell scripts as UTF-8
with BOM for Windows PowerShell 5.1 so authored Unicode labels remain exact. Font
attribute inventory runs only when `-ExpectedFonts` requests an explicit assertion.


On October 25, 2026, the ordinary CLI Release website build passed on Server
2022/T4 without an SDK override. The automatically built SDK was checked against
all 16 extracted SDK inputs, and the mapped executable, cartridge, and host
identities were retained for each run. The frozen package predates the subsequent
shared GLES alternate-view-format fallback; these application results do not
qualify that later source delta.

The named interaction results on that same package are:

- The completed demo portion of a later transport-interrupted run covered the
  calculator keypad and graph controls, Paxflix modal feedback and theme changes,
  and gallery horizontal wheel input. All 104 checks through demo completion
  passed; the complete run remains interrupted.
- A fresh source traversal matched all 72 authored source occurrences exactly in
  all six ordered drawers, with no interior diagnostic click used for acceptance.
  That run recorded 1,177 successful checks but failed before clicking the route
  link: the reveal helper required the footer label to enter the upper 400 pixels
  of the viewport, although it was visible near the bottom at maximum scroll.
  The `/blog` fallback and return-home roundtrip remain unqualified by this run.
  The worker did not time out; the application exited normally and native/GPU
  resources retired cleanly. Preserve this failure when reporting source coverage.
- A separate phase/OpenSource run passed all 54 checks: 90 seconds of stationary
  animation, first and repeated wheel traversals with four settled endpoints,
  1280→720→1280 responsive sizing, actual drag selection and Ctrl+A/C clipboard
  equality, independent code scrolling, tab updates, and drawer retirement.
  Its optional native COM selected-range observer was not run.

On October 26, a fresh route-only replay on that frozen package passed all 39
checks. The footer point was inside both visible label and viewport; `/blog`
mounted the authored 404 page, old native scrollers retired after the exit
animation, and Return home mounted a new outer scroller that responded to real
wheel input. Screenshot review, exact mapped module hashes, normal exit and
native/GPU retirement passed. This boot used a different physical T4 with the
same driver; its raw receipts retain that identity separately. The first attempt
failed a probe live-log sharing read before route input and remains preserved.
The corrected shared reader passed 12 concurrency/error checks before replay.

The October 25 package results above remain separate named results; the later
combined run below does not rewrite their failures or original provenance.
`fullWebsiteAcceptance` remains false for exhaustive whole-site coverage.
Optional Paxflix independent catalog/shelf scrolling, PathStudy native Slider
and Play/Pause controls, and gallery autoplay/hover tests were prepared but not
run before the bounded session ended. Separate October 26 snapping packages
passed 36 checks each in ordinary Debug and workspace-default baked Release
(opt-level `z`, LTO on, one codegen unit), using the ordinary CLI's automatic SDK
without an override. Those packages include the shared GLES view-format fallback;
the website package above predates it. See [the snapping qualification](#native-snapping-and-button-states)
for its exact scope.

The phase run observed visible WinUI at 1.915 seconds and first host publication
at 8.440 seconds after launch, with 200 ms polling. The earlier 0.274-second
process-window observation was a console. The 90.196-second stationary interval
used 88.578 CPU seconds; private bytes rose during the bounded observation, so
this run establishes neither a memory plateau nor a leak. Logged render calls
averaged 7.505 ms, including a 2671.948 ms maximum. These are instrumented call
wall times with diagnostics enabled, not GPU execution or display frame rate;
rolling callback windows may cross phase boundaries.


A fresh October 26 ordinary CLI Release website build included the shared GLES
view-format fallback and allocator/startup diagnostics. The standalone website
profile was opt-level `3`, LTO off, and 16 codegen units; no SDK override was used.
All 16 extracted SDK inputs matched. Captures used Server 2022, WinUI 1.8, driver
596.86 and physical T4 `GPU-268be7a9-1f1f-8b86-4a2b-81540ffdb507`, with exact
executable/cartridge/host/configuration identities retained in each receipt.

That package passed a single uninterrupted **1,213/1,213-check combined matrix**
on October 26, 13:40:19–14:22:19 UTC (42 minutes). Run
`d61eff6f-24f1-4959-8ddb-fded8ee23453` covered the calculator keypad/graph controls,
Paxflix detail feedback and theme controls, gallery horizontal wheel input, all
72 exact authored source occurrences in the six ordered drawers, and the `/blog`
fallback/return-home route. The immutable source and section manifests retained
their original hashes. Independent offline audit confirmed every source click
inside both visible label and viewport, including the one-pixel
`paxflix/movie_shelf.pax` frontier; no interior diagnostic click was used.
The footer point (731,758) was contained, old scrollers retired after 1.033 seconds,
and the new home scroller responded to actual wheel input (0→7.011%). Eight
retained screenshots passed visual sanity review. Application and worker exited
normally without timeout or forced retirement. The final GPU export record had
zero allocated/pending export counters and 103,005 publications/retirements.
All 44 exported payload hashes were verified.

| Website artifact | SHA256 |
| --- | --- |
| `pax-website.exe` | `e0a35581cf6c943f16fc7744316506d5695d120372fe8f172590e5230cf547bc` |
| `pax_website.dll` | `c4c5a3c41fa0b478ac44c9c2c68234f1a9afcb32db3ea24239f75f85b06e888f` |
| `PaxHost.dll` | `808a0ca904df6e25e01f3842769ad1dc020c22faf38e59756daf2c89f858bd29` |
| `pax-app.json` | `57a6e4d7d73f4ffb35b43725c4e506b1857d4b60e303b1dab41d1339a9b64101` |

`websiteInteractionScope=full` and `websiteInteractionAcceptance=true` mean this
named combined matrix passed. `fullWebsiteAcceptance=false` deliberately retains
the broader boundary: optional Paxflix independent catalog/shelf scrolling,
PathStudy native Sliders and Play/Pause, and gallery autoplay/hover-pause/resume
and additional propagation checks were omitted. This mode also skips the separate
traversal/resize/OpenSource drag-selection and Ctrl+A/C sequence; its earlier
54-check result retains the earlier package identity. The new memory traversals
below do not replace selection or resize acceptance. Other devices, gestures,
platforms and exhaustive visual states remain outside the claim. These frozen
binaries predate the subsequent D3D12 cache-allocation-budget and engine
reconciliation-timing changes. Their separate candidate qualification follows.

The subsequent ordinary CLI Release candidate included both changes, retained
the same standalone profile and automatic SDK, and passed all 14 native
contracts before packaging. Its 784-input source manifest contained 778
Windows-relevant inputs with six explicit exclusions; both lockfiles remained
unchanged and all 16 SDK inputs matched. Exact candidate identities were:

| Candidate website artifact | SHA256 |
| --- | --- |
| `pax-website.exe` | `418090c015276967ddd3e4b1128c5aec23d346276311ac23eb0f628c126006a5` |
| `pax_website.dll` | `3c07ba771037b60c14a53bbb9db35dae8a2d3877e25d0680e7b08d9cd7fb9569` |
| `PaxHost.dll` | `3b6ef989e2eeab66a47434072a154c47ce511e407f821ea553bae17af3849520` |
| `pax-app.json` | `57a6e4d7d73f4ffb35b43725c4e506b1857d4b60e303b1dab41d1339a9b64101` |

Candidate run `c49657d6-ca5c-4613-b227-7e6e2391cae9` retained 1,029 passing
checks, the named demo actions, and 61 exact source occurrences before failing
on absolute pointer injection at 16:46:47 UTC. It did not time out. The DCV
receipt records a disconnect at 16:46:38 and a subsequently locked session;
this is temporal correlation, not proof of the exact injection failure cause
or a product defect. The first five source sections were complete; resource
contained three of its 14 occurrences, and the route was not reached. The app
exited normally with 78,539 publications and retirements and zero terminal
export counters. The failed result remains preserved.

A separate bounded replay, `f449cc16-8c0e-4534-b216-dcaf90021c7c`, passed
287/287 checks at 17:31:37 UTC. It repeated the entire 14-source resource section
at original index 5, then exercised the footer route, old-scroller retirement,
Return home, and real wheel response. The isolated section selector restricted
only the expected inventory; source traversal and route bodies remained
identical to the combined probe, including clipped-frontier selection and
exact authored-text hashes. Native parsing and 39 focused controls passed
before input. Independent audit confirmed every source click's half-open
containment and the footer point (731,758) inside its label and viewport. Old
scrollers retired after 2.009 seconds; the new outer scroller moved from 0 to
7.011% with real wheel input. The app and worker exited normally without forced
retirement, with 28,157 publications/retirements and zero terminal counters.
All 77 exported payload hashes and three retained screenshots were reviewed.

The two candidate runs used the same four package hashes, source/section
manifests, and driver 596.86 on Server 2022/WinUI 1.8 at 96 DPI. Their physical
T4 devices were respectively `GPU-d27dabb6-169a-fe0e-886b-40cb2167dc3e` and
`GPU-8aba3f16-736a-37db-c2d7-8de5d192d557`. Exact occurrence union is
**61 + 14 − 3 = 72**, retaining multiplicity and section identity. This qualifies
the named candidate demo, aggregate source, and route scope across two runs.
The residual result uses `websiteInteractionScope=source-sections-and-route`;
it does not convert the failed combined run into a pass.
`singleUninterrupted72SourcePass=false` and `fullWebsiteAcceptance=false` remain
explicit, with the optional interaction and earlier-package selection/resize
limits above unchanged.

The combined run first observed visible WinUI at 1.960 seconds and host publication
at 8.905 seconds, with 200 ms polling. The 0.274-second initial process window was
a console. Startup publication used 900×650 before the 1280×960 test resize;
these are host observations, not display frame timings.

The detailed memory capture passed all 37 checks: two full 90-second stationary
intervals surrounding three down/up round trips, all six settled endpoints, and
normal retirement. Its 376 process samples and 92 supported allocator reports
passed the declared-plan, identity, timing and endpoint completeness checks.
Process private memory rose from 2,065.96 to 3,194.41 MiB over 242.47 sampled
seconds; this establishes neither a leak nor a durable plateau. Allocator reserved
storage reached 1,728 MiB during the first downward traversal and stayed at that
level through the final stationary interval. GPU allocator bytes and process
private bytes cover different storage and cannot be subtracted to infer exact
CPU or driver memory. Stationary animation continued; no globally paused scene
was measured. Exact terminal export retirement passed with 14,434 publications
and retirements; zero exported resources does not mean all allocator storage was
already zero at the earlier close snapshot.

Visible WinUI was first observed at 2.031 seconds and first host publication at
8.991 seconds, using 200 ms polling. Publication occurred at the default 900×650
client size before the measured 1280×960 resize. Logged startup stages included
1,489.949 ms engine initialization, 2,795.058 ms first tick, 78.349 ms first surface
plan, 3,053.828 ms first render and 546.554 ms native apply. These stage and host
observations do not measure GPU completion, presented frames, or settled layout.

Four fresh 45-second stationary processes then passed all 24 checks each in
**disabled/enabled/enabled/disabled** diagnostic order. All four captures and the
cross-run identity/plan checks passed, with normal exact terminal retirement.
The CPU-seconds-per-wall-second observations were 0.9958/0.9749/0.9819/0.9937;
first publication was 8.310/8.692/8.739/8.112 seconds, and private-memory growth was
327.16/344.35/366.34/327.47 MiB. The enabled mode toggled all four diagnostic flags
together. Two processes per mode, uncontrolled OS/driver caches, continuing
animation, and an overlapping background preflight during the third interval
limit this to descriptive observations. That preflight performed two status
reads and a small metadata write; its workload guard rejected before task or
application creation, with no display or input change. These results do not
establish a causal overhead percentage or an allocator-only overhead bound.


### Image request bursts

`asset-queue-contract.cpp` exercises the actual Windows `Assets` implementation
with ten distinct Paxflix thumbnails submitted before polling. It checks active
and queued deduplication, complete draining, synchronous missing-path errors,
repeat requests after completion, propagation and removal of a failed decode,
normal close with queued work, and destruction while work remains. The host retains at most eight active WIC
decodes, with the existing 32 MiB limit per decoded image; additional requests
retain resolved paths until a decoder slot is available. Normal close stops new
requests, discards queued work, and drains only decodes already active. The close
regression queues a corrupt ninth file, calls stop twice, then polls to idle;
the queued file must never begin decoding.

Run on the Windows guest against the SDK source and its already-restored packages:

```powershell
.\build-asset-queue-contract.ps1 -SdkSourceDirectory C:\pax\pax-host\native\windows `
    -OutputDirectory C:\PaxGpuLab\asset-queue-contract -Configuration Release
$env:PAX_ASSET_QUEUE_ROOT = 'C:\pax\examples\paxflix\assets'
$env:PAX_ASSET_QUEUE_OUTPUT = 'C:\PaxGpuLab\asset-queue-contract'
& C:\PaxGpuLab\asset-queue-contract\AssetQueueContract.exe
```

Keep `result.jsonl`, the executable hash, and `assets.h` hash with the result.
This contract test needs no interactive desktop; it verifies image loading, not
native composition or scrolling. The output directory is test scratch space and
contains a deliberately corrupt image for the error-propagation case.

### Native image surfaces

`build-native-image-contract.ps1` builds an interactive `NativeScene` probe and
generates small deterministic PNG/SVG fixtures. It exercises native
`BitmapImage`/`SvgImageSource` loading, asset-root aliases and rejected paths,
shared sources, an eight-operation decode queue, source replacement/deletion,
asynchronous decode failures, and close. Its pixel checks cover centered crop,
contain letterboxing (including downscaling), and stretched SVG alpha gradients
in differently sized consumers after native layout has settled. It also checks
zero-size-before-decode, source-change remeasurement, and resized mask space.
A 10,000-by-10,000 authored SVG records initial native layout dimensions and
checks the final pinned raster bounds and colors; this is not a GPU peak-memory
measurement. Placement, opacity, occlusion clipping, and hit transparency are checked
on the real XAML objects. Native images defer input to Pax raycasting and native
scrollers; they do not take independent toolkit input ownership.

```powershell
.\build-native-image-contract.ps1 -SdkSourceDirectory C:\pax\pax-host\native\windows `
    -OutputDirectory C:\PaxGpuLab\native-image-contract -Configuration Release
$env:PAX_NATIVE_IMAGE_CONTRACT_ROOT = 'C:\PaxGpuLab\native-image-contract\assets'
$env:PAX_NATIVE_IMAGE_CONTRACT_OUTPUT = 'C:\PaxGpuLab\native-image-contract'
& C:\PaxGpuLab\native-image-contract\NativeImageContract.exe
```

Run in an unlocked interactive desktop. Require exit code zero and every row in
`result.jsonl` to pass; inspect `native-image.png` as well. Retain the source and
executable hashes with this evidence. `RenderTargetBitmap` is used only for the
test's pixel readback, never for host presentation. The probe does not establish
desktop website acceptance or broad SVG feature/animated-image support.

For a generated native app, `PAX_WINDOWS_TIMINGS=1` also logs `native_images`
records when source/load state changes. These distinguish mounted, ready,
failed, active, and queued sources and list their resolved asset paths.
`consumers` and `layout_ready` distinguish a decoded source from consumers whose
native layout has settled. The `bitmap_bytes` value estimates first-frame RGBA
storage; it excludes native decoder/GPU overhead and SVG raster storage.


### Native scrolling and rich-text contracts

`build-scene-scroll-contract.ps1` builds an interactive NativeScene regression for
nested `ScrollViewer` axis routing with real mouse-wheel input, native snap points,
repeated and superseded authored writes, collapsed/regrown extents, and callback
retirement after deletion. Its partial patches retain authored pane dimensions
and enabled-axis state, including disabled 0.5/0.25-pixel overflow. Separate
native-turn writes check the latest position and acknowledgement without an
artificial later engine tick. Inspect `firstWriteAcknowledged` in those receipts:
a synchronous first completion does not prove pending-request overlap. A newer
target equal to the current native position must still supersede an older pending
target. The test requires an unlocked interactive Windows desktop.

`build-snap-motion-contract.ps1` covers anchor replacement, clearing, resizing,
and no-motion requests through native mandatory snapping and the public
`IScrollSnapPointsInfo` content interface. Integrated NativeScene scrolling,
mask/input, clipping, retained placement, rich-text, and text-style input contracts,
together with the native snap-motion contract, have passed on Server 2022/T4 with
this adapter. The scroll receipts exercise actual pending-request overlap in both
supersession cases and empty-pane collapse/regrowth. Text-style input checks include
actual taps and selectable Text toggles. The complete native child-control/chrome
matrix also passes, covering defaults and cancellation with visible and captured
controls. The mask and text-style input contracts also pass with the current
TextBox hook. Current ordinary Debug and default baked Release Rust-callback
wheel fixtures each passed 121 assertions across 14 cases with 30 native
observations and normal retirement, using the ordinary CLI and automatic SDK
without an override. Their DX12 cache-budget/startup-subtiming source provenance
is detailed above. The named
[website results](#website-acceptance-workload) retain their separate scope and
route-only replay; earlier ScrollView Debug/Release application results do not
qualify that boundary on this replacement.

Isolated visible/captured ScrollViewer checks observed same-axis boundary
consumption: a wheel further past the inner start or end moves neither viewport,
while reversing direction moves the inner viewport. Do not conflate that behavior
with routing an axis whose native extent is zero. Touchscreens, precision
touchpads, and open native popups need separate qualification.

`build-placement-contract.ps1` exercises retained per-node transforms and
rectangular clips without skipping the placement pass. It checks empty updates,
child and ancestor movement/rotation, reparenting, clip bounds and toggles,
autosized text remeasurement, and deletion/recreation using actual XAML object
identity and matrix values. Edge-on parent and clipped-node cases require paint
and input suspension without losing editor text or selection, then recovery when
the transform becomes invertible. Set `PAX_PLACEMENT_CONTRACT_LOG` for JSONL output.
A retained transform must still recompute the child's relative matrix whenever
its ancestor changes; child-only dirty flags do not establish this contract.

`build-native-mask-input-contract.ps1` checks the production mask/input path on
an unlocked interactive Windows desktop. Set `PAX_NATIVE_MASK_INPUT_LOG` for
JSONL evidence. Its real wheel and button input distinguishes visible interiors,
opaque holes, disconnected gaps, rounded corners and nearby curved boundaries,
ancestor clips, and transformed nodes. Screen pixels independently verify that
opaque clipping hides covered native controls. Same-node capture-to-opaque-to-clear,
zero-alpha, resize, and autosized-text cases check branch transitions and clip
retirement. A collapse-to-zero and regrow case retains the retired input Path and
requires its Geometry to be detached before reuse, independently of deferred
compositor lifetime. This integrated NativeScene contract has passed on Server
2022/T4, including mouse-wheel input with the captured source hidden and a
complete rerun with the current TextBox wheel hook. The native child-control/
chrome matrix also passes with visible and captured controls. Ordinary Debug
and default baked Release Rust-callback wheel fixtures each passed all 121
assertions across 14 cases with 30 native observations and normal retirement,
including fixed vector-overlay ownership outside observed native text. Both used
the ordinary CLI and automatic SDK, with current source provenance detailed
above. The
[website results](#website-acceptance-workload) separately record completed source
coverage and the separate route-only replay.
A run interrupted by a locked or disconnected desktop does not count as a pass.
Capture input checks do not establish correct fractional color blending.

`build-rich-text-contract.ps1` consumes the shared
`pax-chassis-common/src/desktop/rich-text-cases.json` fixture through
`PAX_RICH_TEXT_FIXTURES`. It checks exact Unicode and newlines, genuine native
runs/colors/styles, hyperlink grouping and host navigation, actual selection and
link focus across style-only patches, zero-font recovery, and explicit malformed
input rejection. Set `PAX_RICH_TEXT_LOG` to capture its JSONL result. These focused
checks complement the unchanged website's real source-drawer and scroll probes;
they do not establish touch, IME, or full accessibility parity.

The interactive harness must verify the actual foreground HWND before injecting
input. Windows can deny `SetForegroundWindow` after a prior test process exits;
use the existing probe's ownership-checked titlebar click to activate only the
unoccluded test window. Preserve failed preconditions separately from semantic
failures. The scene-scroll contract records native extents and offsets around its
wheel input so readiness and route failures can be distinguished.
For title assertions, use the fixture's active route metadata: the website
updates its native window title after initialization. Its packaged `pax-app.json`
does not contain that route title, and the host's initial `Pax` default is not a
valid title oracle once the home route has mounted.

Discover the actual native ScrollPattern and record its UIA runtime ID when
qualifying a scrollable peer. Earlier ScrollView receipts used its template
ScrollPresenter; those peer identities must not be reused for the ScrollViewer
replacement. Do not assume an outer control's Pax AutomationId is forwarded to a
template peer or change production IDs to satisfy the harness.

For the website's long source drawer, selection acceptance uses an actual pointer
drag followed by Ctrl+C, then native Ctrl+A/Ctrl+C and exact Unicode/newline
comparison. The installed Windows UIA client, explicitly launched with `-STA`, has separately crashed in
`RawTextRange_GetText` when querying the selected range after programmatic
selection of this document; a passing keyboard-copy check does not qualify that
accessibility path. Earlier editor and selectable-Link checks reproduced the
managed failure in both STA and MTA while native COM MTA reads passed. On the same Server 2022/T4 runtime, `-NativeSelectionExecutable` now runs
`selection-uia.exe` against this exact rich document after actual pointer drag
and Ctrl+A. Native COM MTA readback passed for both the 144-character selection
and the complete 3,928-character document at bounded `4096` and `-1` lengths.
This isolates a working native client path; the managed client crash remains
unresolved and is not evidence that changing apartment alone fixes it. A supervisor must still close the test app if the checker
terminates in native code before its PowerShell `finally` block runs.

### Orthographic spatial fixture

Build `examples/fixtures/spatial-workbench` for Windows through the ordinary CLI,
then run `check-native-spatial.ps1 -Executable <packaged-spatial-workbench.exe> -OutputDirectory <fresh-directory>` in an unlocked interactive desktop, under an
external ten-minute process deadline. The probe needs a working area of at least
1340 × 1010 pixels. Repeat with Debug and baked Release packages.

The probe sends real mouse/keyboard input to transformed controls, checks edited
value and native identity across zoom and clipping recovery, and captures the
composed window for visual review. It also checks the explicit orthographic
fallback when the application selects unsupported perspective, and normal GPU
export retirement. A passing report requires visual review of its captures;
it does not qualify perspective, occluded hit testing, physical input latency,
or frame rate. The lower-level placement contract separately covers retained
controls through singular transforms.

## Text input callback ordering

Build `build-text-input-contract.ps1` against the same native SDK source and
run `TextInputContract.exe` in an interactive desktop. Set
`PAX_TEXT_INPUT_CONTRACT_LOG` to a fresh JSONL evidence path and require exit 0.
The contract checks synchronous native edit delivery, consecutive edit order,
selection-preserving identical echoes, silent application replacements (including
after the dispatcher advances), and event revocation after deletion. It uses
native TextBox setters to isolate callback ordering; `check-native-spatial.ps1`
separately supplies real keyboard input to the packaged Rust fixture. Qualify
Debug and Release independently. Neither test establishes additional IME coverage.
