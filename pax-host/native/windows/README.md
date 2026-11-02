# WinUI desktop host

This desktop-alpha host connects a user-owned Rust `main`, a Pax desktop cartridge,
real WinUI controls, and the shared Pax GPU renderer. It is a standalone host;
embedding through `mount` returns an explicit unsupported error. One WinUI
`Application.Start` is supported per process.

The qualified environment is Windows Server 2022 on an EC2 Tesla T4. This alpha
status does not establish client Windows 10/11, physical input latency, or
long-session memory stability.

The host establishes per-monitor-v2 DPI awareness before creating UI and rejects
an incompatible awareness mode already selected by the application. An
executable manifest is preferred for packaged launchers; direct Cargo launchers
also need the Windows App SDK's supported-OS declaration as part of their app
configuration.

On October 18, the Server 2022/Tesla T4 pilot passed 49 application checks and
61 keyboard checks at actual 125% display scaling (120 DPI), with
`TextScaleFactor=1`; the original 96 DPI and text scale were restored afterward.
This establishes that fixed display scale, not live or multi-monitor DPI
transitions. Separate 150% and 200% text-only scaling checks exposed clipping in
the fixture's fixed 30-pixel-high labels; explicit heights remain fixed when
native glyphs enlarge. Autosized text is covered separately by the
`native-text-layout` fixture and its native/engine measurement checks.

For Text measurement, omitted axes use XAML's automatic dimension during
`Measure`; explicit dimensions, including zero, remain constraints. The host
places the measured bounds and returns the engine's request generation.
A Text font size of zero suppresses its inner native glyph block while retaining
its content, identity, and explicit authored box. Unconstrained axes measure zero;
a later positive size restores native text and fresh measurement. This avoids
WinUI's rejection of `TextBlock.FontSize(0)` without drawing a minimum-size glyph.
This zero-size adaptation applies to Text, not native form-control typography.
`UISettings.TextScaleFactorChanged` coalesces a native-metrics invalidation
onto the UI thread's next accepted batch, including an empty batch in a static
scene. The engine keeps the previous bounds until a fresh matching response
arrives. Repeated Text nodes own separate measurement state, and request tokens
survive remounts so late replies cannot replace current measurements.

On October 19, the generated `native-text-layout` fixture passed cold launches
at actual 100%, 150%, and 200% accessibility text size in both Debug and baked
Release on Server 2022/Tesla T4, at unchanged 96 DPI. Live checks also passed:
Debug 100% → 150% → 200% and 150% → 100%, and Release 200% → 150%.
Each transition retained its process and observed layout before content edits
or window resizing. Engine/native bounds, paragraph terminal-character ranges,
autosized Stacker/Frame layout, independent repeated Text nodes, and unchanged
explicit dimensions passed; every process closed with all GPU exports retired.
The fixed-height label deliberately exposes clipping at 200%. These checks
establish this fixture's line layout, not pixel-level glyph visibility,
screen-reader behavior, mixed-DPI transitions, or client Windows qualification.

`pax-chassis-windows` requires a WGPU-compatible DX12 integrated or discrete GPU.
It does not silently fall back to WARP. Native composition uses GPU copies into
`CompositionDrawingSurface`; presentation never reads the rendered pixels back to
the CPU. The runtime's native mask paths drive both compositor alpha masks and
input geometry. TextBox editing remains in the real control.

Standalone Scroller corner radii and fully opaque vector coverage use a native
geometric clip on the original control visual. Matching input paths carry both
XAML geometry and compositor geometry, so a transparent path's bounding box does
not stand in for its actual coverage. These cases avoid a mask bitmap or redirected
capture. Zero-opacity coverage is ignored. Clip geometry is rebuilt when native
measured or explicit bounds change.

Fractional coverage still uses a redirected visual and alpha-mask surface. Its
rounded clip belongs to the displayed capture instead of being applied twice.
The experimental `ScrollViewer` adapter preserves native mouse-wheel input with
the captured source hidden. Its integrated NativeScene mask/input contract has
passed on WinUI 1.8, Windows Server 2022, and an NVIDIA T4. Native child-control
and scrollbar defaults/cancellation also pass with visible and captured controls.
Ordinary Debug and default baked Release Rust-callback wheel fixtures each
passed all 121 assertions across 14 cases with 30 native observations, including
fixed vector overlays outside observed native text bounds and normal retirement.
Both used the ordinary CLI and automatic SDK without an override, including the
DX12 cache-budget and startup-subtiming changes. Full website acceptance remains
a separate gate. The work is tracked in
[PAX-1045](https://linear.app/paxdev/issue/PAX-1045/preserve-windows-native-scrolling-under-fractional-compositor-masks);
no synthetic scrolling or epsilon-opacity workaround is used.
The rounded boundary survives ordinary reactive updates and restoration after a
zero-sized viewport. Alpha-mask/capture allocations remain separate from the
export budget below.

The initial Windows target retains the alpha-only native punch-through used by
the existing web and Apple targets. GPU content sits behind the alpha-masked
native visual, so partially transparent foreground vectors can contribute too
little color over native controls. This known approximation is accepted for the
initial target; exact color correction is deferred to
[PAX-1040](https://linear.app/paxdev/issue/PAX-1040/correct-translucency-edge-cases-in-compositor).
Opaque masking and input ownership checks do not establish fractional blend
fidelity.

## Lifetime and resource bounds

Three export batches retain their images until the native copy fence completes.
Each batch contains at most 64 images. The export budget defaults to 256 MiB and
can be set with `PAX_WINDOWS_EXPORT_BUDGET_BYTES` before creating the engine. It
counts DX12 resource allocation requirements across cached and outstanding export
generations, rather than only image pixel bytes. It is not a process RSS or
compositor memory limit.

Long animated opacity scenes still have an alpha limitation: process private
memory can grow while the live export and scene counts remain bounded. The
`neon-opacity` animation/pause/resume checks observed growth during animation
and a flat or slightly declining paused phase. The shared buffer and bounded
capture fixes reduce specific GPU allocation churn, but do not establish
long-session memory stability on this Windows host.

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
reported as a failure, never as clean retirement. Native affine patches require
six finite numbers; invalid components report the control kind, node ID and
component index instead of falling through to a generic WinUI parameter error.
Loaded cartridge code remains
process-pinned because Rust/driver callbacks and thread-local teardown can outlive
an individual resource session.

Dropping a prepared application before `run` is a separate boundary: no WinUI
dispatcher loop exists yet. The host pumps the original STA's messages and
cleanup wakeups without creating a window, waiting for at most the 15-second
cleanup deadline. A timeout or quit request reports failure and retains unfinished
ownership. Normal window close remains asynchronous. The smoke fixture's
`--prepare-drop` control checks repeated preparation and disposal without a GPU;
`--prepare-drop-async` additionally requires delayed worker acknowledgement.

The host coalesces asynchronous wake requests into one low-priority dispatcher
callback while retaining its regular frame timer and preparation wake event.
This prevents a busy scene's wake queue from starving native input, resizing,
and closing. Callback/publication timings are not display frame-rate measurements. With
`PAX_WINDOWS_TIMINGS=1`, `host_startup_stage` separates cartridge loading, engine
initialization, window/GPU/composition setup, and first-frame tick, plan, render,
apply, commit, and publication. These elapsed host spans do not measure GPU
execution time or guarantee that autosized text has settled.

The same flag emits `windows_startup_tick` for the first three engine ticks and
`windows_backend_init` for the first 64 GPU backend constructions in the process.
The tick stages separate property import, node updates, handlers, initial geometry,
proximity delivery, and native culling. Post-proximity effects, geometry, and
occlusion share one span, with the number of compositing reconciliation passes
reported separately. `windows_startup_reconciliation` splits that group into
per-iteration effect-drain, geometry, and occlusion spans with stored work-counter
deltas. The effect-drain span wraps `drain_node_effects`, including property-graph
entry and shared-value import. With `PAX_EFFECT_DRAIN_INSTRUMENTATION=1`, the separate
`PaxEffectDrain` elapsed time covers the instrumented drain and excludes that
entry/import and subsequent diagnostic formatting. Its top effect counts are
execution counts, not per-effect durations; subtracting the two spans does not
isolate import time. The reconciliation log buffers at most 17 records, including
the final clean check or a failed
pass-limit check, and writes them after the work. An absent occlusion span means
that call did not occur; `unattributed_ms` preserves the iteration's remaining
cost. The enclosing group also includes initial observation and record insertion.
An unsettled or panicking record is not successful startup evidence; an aborting
panic may produce no final log. Backend stages separate buffer setup, pipeline families,
and initial targets; a pipeline-family span includes cache lookup, shader-module
and layout creation, and pipeline creation, rather than just driver compilation.
These spans nest inside the host timings; they must not be added to them. Construction counts do not
count physical devices or presented frames. Both diagnostic paths are compiled
only on Windows and are disabled by default.

`PAX_GPU_MEMORY_DIAGNOSTICS=1` adds an `allocatorReport` to Windows GPU status.
It reports live wgpu allocator allocations, reserved blocks, unused reserved bytes, and
empty blocks for that device. The allocator walk is cached for at least one
second; `sampleUnixMs`, `sampleAgeMs`, and `sampleDurationMs` identify sample time,
freshness, and allocator collection/summarization cost. That duration excludes
copying the cached report and serializing or writing the enclosing status log.
The largest 16 allocation-label groups are
included, with labels limited to 160 characters and totals for omitted groups.
Its `blockReport` also shows up to 16 allocator blocks, each with its eight
largest label groups, live bytes, unused reservation, and omitted-group totals.
Omitted blocks retain aggregate totals. Check `consistent` before interpreting
these figures: it reconciles allocation ranges, byte offsets, and overall totals;
unknown sums are `null`. Block indices identify entries in one snapshot, not
stable block identities, heap types, or VMMap addresses. This extra summary runs
only when the enabled diagnostic sample refreshes, and its cost is included in
`sampleDurationMs`.
An unsupported report has `supported: false` and no numeric allocation totals.
Reserved slack includes padding and fragmentation; it does not establish a
leak. `supported: true` means the report is available, not that it covers all GPU
storage. Allocations that bypass this allocator, driver/pipeline/descriptor
storage, WinUI/native-composition allocations, and whole-process private bytes
are outside its scope. Compare those separately, match sample timestamps, and compare
with diagnostics disabled before drawing performance conclusions.
The `closed_gpu` snapshot is collected before the engine is destroyed, even
though the host writes it afterward. Export counters must drain to zero, but
this snapshot can still include renderer caches and a cached allocator sample;
allocator totals need not be zero at that point.

The retained vector-buffer cache charges D3D12 allocations using each buffer's
64 KiB allocation granularity and the pinned wgpu version's internal padding.
Upload-byte counters remain separate. This bounds unused geometry retention
more closely than payload sizes alone; live scene references can keep the cache
above its target. It does not limit allocator heap reservations, native surfaces,
or total process memory. The padding assumptions must be rechecked when wgpu is
upgraded, using the ignored DX12 allocation-report test in `pax-gpu`.

`PAX_WINDOWS_PHASE_TIMINGS=1` adds cumulative and interval wall-time and current
callback-thread CPU totals for each host stage. Windows thread accounting can
quantize small samples; use accumulated windows. Worker threads, GPU execution,
and WinUI work outside the callback are excluded, while phase wall times include
response decoding and diagnostic bookkeeping. The optional measurements add
overhead; compare the same binary with the flag disabled. See the
[native probe guidance](../../../tests/src/native-platform-probes/windows/README.md)
for the phase list, checkpoint cadence, and website acceptance procedure.

## Assets and controls

The initial application path implements Text, Textbox, Button, Checkbox, Slider,
Dropdown, RadioList, Frame, Scroller, and EventBlocker. The new form controls use
WinUI Slider, ComboBox, and RadioButtons. Programmatic updates suppress native
input callbacks; option replacement retains its requested index, with no
selection or sentinel event for an empty list or invalid index. Authored slider
values clamp to the finite range, while user input applies the positive step
anchored at the minimum. Native dropdown popups are separate surfaces and need
their own transformed-placement and clipping qualification.
Read-only Markdown and the supported source-highlighter HTML subset are normalized
by the shared desktop parser into native TextBlock runs and Hyperlinks. Native
selection, Unicode, range colors, emphasis, monospace spans, relative heading sizes,
and preformatted line height are retained. Adjacent authored links keep separate
keyboard stops; formatting within one link shares its native Hyperlink. Style-only
updates retain runs, selection, and link focus. Hyperlinks use the host navigation
path, including virtual routes. Editable Text and unsupported markup return explicit
errors; unsupported content is not silently flattened. Additional native control
families also return explicit unsupported errors. This is not a claim of full
native-target parity, including touch, all IMEs, pointer capture,
accessibility navigation, or every styling property.
Button applies authored `color`, `hover_color`, `outline_stroke_color`, and
`outline_stroke_width` through its native template brushes. Hover color also
supplies the pressed fill, and text fill persists in those states. Reactive
updates mutate retained brushes without replacing the native control. Disabled
and high-contrast state parity are not qualified by the normal/hover/pressed
pixel checks.

Finite negative Scroller inner-pane dimensions become zero-sized native extents,
matching ordinary native control sizing when responsive parents collapse. Positive
updates restore the pane without replacing its native scroller. Nonfinite extents
remain errors; authored values and shared layout are not rewritten.

Scroller uses an experimental WinUI `ScrollViewer` adapter. It keeps authored
pane dimensions and axis enablement separate from the native content dimensions.
An authored-disabled axis has no native scrollable extent, including the shared
0.5-pixel overflow tolerance. The original mouse-wheel event can then continue to
a native ancestor without translating vertical input into horizontal movement.
The host does not synthesize another wheel event or scroll an ancestor manually.

The legacy control currently consumes same-axis wheel input at either end of a
nonempty viewport: the ancestor stays still, and reversing direction moves the
inner viewport again. Isolated checks observed this with both visible and
captured controls. It differs from declining an axis with no travel. Touchscreens,
precision touchpads, open native popups, and uninterrupted inertia under delayed
engine echoes remain unqualified.

Snapping uses the public `IScrollSnapPointsInfo` content interface with native
mandatory policy and near-edge alignment. Anchors stay in local logical pixels;
effective points are clamped to the scrollable extent, sorted, and deduplicated.
An empty list clears snapping; omitted lists retain the existing policy. Actual
anchor, extent, or enabled-axis changes apply after native placement and layout,
while unchanged position echoes do not force layout or retarget the view.
Off-anchor writes acknowledge the actual native offset even when snapping produces
no movement. The host serializes accepted authored requests, retains the latest
queued target, and waits for the native final event before submitting that target.
A no-motion `ChangeView` return has no `ViewChanged` event and needs an explicit
actual-position acknowledgement. Native snap and integrated NativeScene checks
pass on Server 2022/T4, including overlapping authored requests, no-motion
acknowledgements, and collapsed/regrown extents. On October 26, fresh
`native-scroll-snap` packages also passed all 36 checks in both ordinary Debug
and workspace-default baked Release (`opt-level="z"`, LTO, one codegen unit),
using the CLI's automatic SDK without an override. Final-core reruns included
the DX12 cache-budget and engine-subtiming changes; their 784-input audit matched
778 relevant inputs and 16 SDK inputs. Those runs covered both axes,
dynamic anchors, resized viewports, repeated off-anchor writes, native/shared
offset agreement, Button states, and graceful shutdown with complete
exported-image retirement at 96 DPI. Touch/trackpad momentum and client Windows
versions remain outside that qualification.

Native movement sends ordinary Scroller position acknowledgements and Pax
`@scroll` notifications. The raw-wheel bridge delivers each original event once
before native defaults, so a Pax `@wheel` handler can prevent that event's default
handling. Native child controls receive an earlier hook where their own class
handler would otherwise run first. This implementation addresses
[PAX-1046](https://linear.app/paxdev/issue/PAX-1046/deliver-raw-pax-wheel-callbacks-from-winui-scrollpresenter-input);
NativeScene scrolling, mask/input, clipping, placement, rich-text, and text-style
input contracts have passed on Server 2022/T4. Native child-control and scrollbar
checks pass for Slider, ComboBox, TextBox content and its scrollbar, both Viewer
axes, and empty viewports, covering defaults and cancellation with visible and
captured controls. On October 25, the packaged baked Release Rust-callback fixture
passed 121 assertions across 14 cases: exact once-only bubbling, both axes,
window-space input, root cancellation, rounded ownership, fixed vector overlays,
and native/bound offset agreement. It used a pinned SDK override and an explicit
opt-level 3/LTO-off/16-codegen-unit profile. On October 26, fresh ordinary Debug
and workspace-default Release wheel packages each passed the same 121 assertions
across 14 cases with 30 native observations and normal retirement. They used the
ordinary CLI and automatic SDK without an override, including the DX12
cache-budget and startup-subtiming changes. Fresh snapping fixtures on that same
final core also passed 36 checks each in ordinary Debug and workspace-default
baked Release (opt-level `z`, LTO on, one codegen unit), using the ordinary CLI
with its automatic embedded SDK and no override. They passed exact mapped-module
and terminal-retirement checks; earlier morning snapping receipts retain their
separate source provenance. October 26 ordinary CLI
Release website packages used the automatic SDK without an override and the
standalone opt-level 3/LTO-off/16-codegen-unit profile. The package including the
D3D12 cache allocation-budget correction and startup reconciliation diagnostics
has audited aggregate coverage for embedded demos, all 72 exact source
occurrences across six ordered drawers, and footer-route retirement and
return-home wheel response. Its combined run retained 1,029 passing checks and
61 source occurrences before pointer injection failed following a DCV
disconnect; that run remains failed. A separate resource-section/route replay
passed 287 checks, repeating the entire 14-source resource section. Exact union
coverage is 72 occurrences with three repeated, across two processes and
physical T4 devices. Both apps exited normally with exact terminal retirement.
The earlier package passed 1,213 checks in one combined run. Separate
earlier-package results cover selection, clipboard copy, code scrolling, resize,
and timed traversal. The
[probe guide](../../../tests/src/native-platform-probes/windows/README.md#website-acceptance-workload)
records exact provenance and omitted optional interactions. Aggregate candidate
coverage does not establish a single uninterrupted pass or exhaustive
whole-website coverage.
Earlier IME and display-scaling results retain their original host provenance.
Default prevention does not cancel momentum already underway or make `@scroll`
a before-default event.

Native placement retains one `MatrixTransform` per node and reuses its rectangular
clip geometry. Every placement pass still recomputes each node's matrix relative
to its current parent, updating the native matrix when a component changes. This
preserves ancestor movement, reparenting, and autosize behavior while avoiding
new transform and clip objects for unchanged geometry. The focused
`build-placement-contract.ps1` regression checks actual native identity and values.

Plain Text uses a genuine `TextBlock` inside a full-box native Grid. The Grid
preserves the authored capture/mask coordinates while the intrinsic text block
supports independent horizontal/vertical placement and multiline alignment.
Native underline can be changed or cleared. No-wrap text can extend beyond an
unclipped box; `clip=true` applies an explicit clip at the authored bounds.
UI Automation reports the intrinsic text peer, whose bounds need not fill an
explicitly sized Pax node. Omitted dimensions still use generation-checked native
measurement. RadioList retains genuine `RadioButton` items and applies text
font/fill to existing and newly inserted choices. Native controls also receive
content alignment, and Textbox receives multiline text alignment. Button,
Dropdown, and RadioList retain native TextBlock labels for reactive underline;
Dropdown retains each label when WinUI moves the selected item into its faceplate.
Textbox underline remains unsupported so ordinary native editing and IME behavior
are preserved.

The October 21 text-style fixture passed 121 checks each in Debug and baked
Release, including native popup Escape/focus, selection commitment, reactive
styles, and editor-content retention. After this Text representation change, Debug autosizing passed at
100% and through a same-process change to 150%. Returning to 100% passed layout
and clean-close assertions, but its final screenshot lost foreground ownership.
The earlier October 19 Release scaling matrix has not been repeated with this
representation.

Relative `Navigate` requests targeting the current window update a per-session
virtual `pax://app/` location through the existing Router interrupt. Native routes
follow the Apple coordinator's path/query/fragment semantics and do not provide
browser history. Public absolute destinations use `Launcher::LaunchUriAsync`;
private `pax:` destinations are never handed to the OS. Empty destinations are
ignored. The external application handoff still needs an interactive qualification.

Selectable native Text uses XAML's Tapped gesture for Pax Click, preserving drag
selection without activating an enclosing Link. Nonselectable Text retains the
ordinary canvas pointer path. This does not make arbitrary native control default
behavior cancellable through Pax pointer events.

The default Roboto regular/weight/italic faces ship as desktop application assets.
Debug, baked Release, and direct Cargo navigation checks verify actual glyph
metrics against the exact staged font files and exclude Arial/Segoe UI fallback.
Direct Cargo stages the embedded fonts and their license notice beside the executable.

The canvas has its own keyboard focus target. Clicking vector content returns
focus there; native editors and controls retain their editing and IME behavior.
Canvas key events preserve repeat and modifier state, release held keys when
focus leaves, and honor default prevention. `KeyPress` covers printable keys and
Enter after an uncancelled `KeyDown`, excluding Control/Windows shortcuts.
Pointer modifiers use the shared Shift, Control, Alt, and Command names.

Cursor requests use the corresponding Windows system shape where one exists.
CSS-only shapes such as zoom, copy, and alias, and custom cursor URLs, fall back
to Arrow. This is not complete browser cursor-shape parity.

Local images decode through WIC into straight RGBA for the runtime's image API.
At most eight decodes are active; additional requests queue resolved paths,
with duplicates coalesced until completion. The 32 MiB decoded RGBA limit is
per image. The queue does not impose a scene image-count limit or bound the
runtime's retained image memory. Shutdown waits for active decoders and
discards queued work.
This asset upload is separate from presentation. Local fonts resolve against
ordered `assets` roots and the corresponding `fontAssets` roots staged beside the
executable. A font with an empty URL selects a system family. Remote font CSS and
remote images need an explicit download policy and are not supported here.

## Building and qualification

`build.ps1 -Architecture x64 -Configuration Debug -OutputDirectory <absolute>`
builds `PaxHost.dll` and its runtime dependencies using MSVC v143 and pinned NuGet
packages. ARM64 and Release are also available. A compatible Windows App Runtime
must be installed. Preserve the complete SDK output directory, including bootstrap
runtime dependencies. The CLI keeps SDK symbols and linker artifacts there while
omitting them from Windows Release packages; shipped runtime DLLs and resources
must stay together. Cargo/CLI integrations build a scratch copy of authored
sources so NuGet caches do not become embedded SDK input. The build script handles
Rust's canonical extended drive and UNC spelling for PowerShell's filesystem
provider; this does not establish support for arbitrary long paths or network
build environments.

For a same-package application library and launcher, CLI packaging freezes the
selected cartridge before the launcher build can overwrite Cargo's shared PDB
outputs. Optional application symbols are preserved in the sibling
`.pax/build/<debug-or-release>/windows-symbols/{cartridge,launcher}/` directories;
the runtime package does not include them. This does not change Cargo's output
warning or qualify separately packaged cartridges. Use a profile retaining debug
information when private function names or source lines are needed. See the
[desktop packaging guide](../../../pax-docs/book/src/targets-build-deploy.md#build-and-run-a-desktop-package)
for the package and diagnostic artifact layout.

The dynamically loaded host initializes the Windows App Runtime explicitly during
configuration, after the DLL loader lock is released. Bootstrap failures return
through the host error API. The runtime dependency remains valid for the
process-owned WinUI host's lifetime; DLL static initialization and teardown do not
perform bootstrap or COM shutdown.

Set `PAX_HOST_SDK_DIR` to the built SDK directory to use it when building the
`native-desktop-smoke` fixture. Direct Cargo loads the SDK from that directory;
packaged CLI output places the required SDK files beside the launcher. With no
override, the CLI builds the native SDK automatically in the project's `.pax`
directory. These are distinct launch gates, especially for resource resolution.

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
composed image pixels, and clean export retirement. Use `-WorkingDirectory` to
exercise a relocated package from a directory unrelated to its assets. Optional
`-SoakSeconds 180` samples process resources while repeatedly resizing and
animating controls, checks declared GPU export bounds, and records host publication
cadence; that cadence is not display or remote-desktop FPS. Add
`-SoakWithoutResize` to isolate animation at a fixed window size when comparing
memory growth. The export bounds do not bound process private memory or native
compositor caches. The checker records explicit exclusions instead of treating
keyboard injection as IME qualification or a
requested font-family property as proof of the resolved font face. Build the
SDK-only helper with `build-font-metrics.ps1` and pass its executable through
`-FontMetricsExecutable` to compare the label's native text-range widths against
DirectWrite shaping from the exact staged Manrope file. The full line and
repeated `@` glyph range must match Manrope and exclude distinct Arial and Segoe
UI negative controls at the same size and shaping context.
An ambiguous staged font path
requires explicit `-StagedFontFile`. This additional gate must pass before
claiming bundled-font resolution. The older
isolated composition checker and its strict managed-UIA default are unchanged.

`check-native-keyboard.ps1` uses the fixture's visible event counters to verify
canvas focus, down/up/press ordering, default prevention, repeat, focus-loss
release, native editing isolation, pointer modifiers, the system hand cursor,
and Tab navigation. It checks the named `Enter`, `ArrowLeft`, and `Escape` keys
and their `KeyPress` behavior, then verifies that an actual activation change to
an owned window releases a held key without a duplicate release on return.
For close, it keeps F12 physically held until the app exits and requires exactly
one release marker from the actual Rust `KeyUp` handler, alongside clean export
retirement. These checks passed in both Debug and Release on Windows Server 2022
with a Tesla T4. Keyboard injection and the separate translator's dead-key lookup
tests do not qualify a real IME session.

`check-native-ime.ps1 -Executable <fixture.exe> -ImeUiaExecutable <ime-uia.exe>
-OutputDirectory <evidence>` launches its own fixture and observes genuine
Microsoft Japanese IME composition through native UI Automation. Build the
read-only helper with `build-ime-uia.ps1`. Run the checker in a hidden, STA
PowerShell process on the interactive guest desktop: a visible runner console
can steal focus and commit composition. Supervise the complete checker with an
external deadline; its helper's timeout does not bound managed UIA calls. The
guest must already have only the US English and Microsoft Japanese input
profiles, with English active initially;
the checker installs nothing and changes no language-list or default-profile
settings. It sends guest keyboard events, verifies the native preedit range and
candidate list, commits the observed conversion, checks selection and cancellation,
and requires native input to stay out of the canvas handlers. It also closes the
fixture during verified active composition and checks clean export retirement.
Success requires restoring the original English keyboard layout on an owned
temporary window after close; cleanup also attempts restoration on failure.
UTF-8 results, flushed UIA observations, host logs, and desktop screenshots
remain in a unique evidence directory.

Add `-CheckGeometry` to compare candidate placement with the composition range
after moving the window, rotating the fixture's upper editor by three degrees,
and scrolling both native scrollers before composing in the inner editor. This
requires enough desktop space for the 1000-by-840-pixel window and a visible
translation. On October 18, this standalone checker with `-CheckGeometry` passed
all 62 assertions against the generated Release fixture on Windows Server 2022
with a Tesla T4 and the installed Microsoft Japanese IME. This included active
composition immediately before close, clean export retirement, and restoration
of the exact original English keyboard layout. Debug IME execution remains
unqualified. These checks cover that installed IME in romaji mode; they do not
establish other IMEs, client Windows 10/11, reconversion, screen-reader behavior,
or composition continuity while a scroll or transform is actively changing.
Layout restoration does not restore remembered IME conversion-mode preferences.

The same Release package passed all 49 application checks from a fresh,
non-administrator Windows user profile, an unrelated working directory, and a
PATH containing only Windows tools, with Pax SDK and Rust environment overrides
cleared. The package loaded Windows App Runtime 1.8 (`8000.946.1701.0`) from
WindowsApps and VC++ runtime `14.44.35211.0` from System32, without installing
additional prerequisites for that user. This qualifies a fresh user on the
prepared Server 2022 image; it does not qualify installation on a clean machine
without those machine-wide runtimes. Keep the runtime DLLs, PRI, and assets in
the packaged directory together.

`check-native-reload.ps1 -Cli <pax-cli.exe> -Project <fixture> -OutputDirectory
<evidence>` launches the actual Debug CLI, changes and restores one template
label, and requires the same native process/window throughout. Its shell must
already have MSVC, Rust, and `PAX_HOST_SDK_DIR` configured. Source restoration is
verified by hash even when a check fails.

Set `PAX_WINDOWS_HOST_LOG` to an owned file for GPU and lifetime diagnostics.
Set `PAX_WINDOWS_TIMINGS=1` before launching for bounded host timing summaries.
Callback work, callback intervals, and successful publication intervals retain
their last 4,096 samples and report p50/p95/p99/max at shutdown. The total sample
count and retained window are distinct. These measurements do not wait for GPU
execution or display presentation and are not DCV playback frame rates.
The same opt-in reports live rounded clips, alpha-mask pixel storage, and
redirected capture counts. Mask bytes are a logical pixel estimate; captures
are counted without estimating their driver storage. Neither replaces an
independent measurement of total compositor/GPU memory.
Hardware/UI qualification is still required for changes to this host; a source
build alone establishes neither native input behavior nor composed-pixel fidelity.

## Receiving a generated application

The October 21 default `pax-cli create` project passed Debug and baked Release
interaction checks without changes to its generated Pax/Rust source. The
unpublished branch used a recorded Cargo path override for Pax crates. A final
ordinary `pax-cli build --target windows --release` built its own SDK with no
`PAX_HOST_SDK_DIR` override or manual DLL replacement; all 15 authored Windows
SDK/common ABI inputs matched the generated interface. This checks the default
SDK build while the Pax source tree is available, not the absent-source embedded
fallback.

That untouched 16,820,590-byte package passed the 25-check starter gate under a
fresh Users-only account, from an unrelated working directory with a
Windows-only PATH and source/compiler access denied. The gate verified actual
loaded executable/cartridge/host hashes, native Slider input, Replay, reactive
background changes, exit zero, and GPU export retirement. The machine already
had Windows App Runtime 1.8 and the Visual C++ runtime installed; first launch
registered the existing App Runtime framework for the new user. This qualifies
a receiving user on that prepared machine, not prerequisite installation on a
clean Windows image.
