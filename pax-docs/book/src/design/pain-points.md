# Pain points

Example (structure is not rigid but this showcases the style of notes we should be gathering.

## 2026-05-19

Tried to do `x`, ran into `y`, solved it by `z`.
Recommendations:  (e.g. document in pax-docs, document in API docs (inline comments), update AGENTS.md, create a more targeted example, consider redesigning APIs, etc.)

## 2026-05-20

Mobile WebKit requires `DeviceOrientationEvent.requestPermission()` /
`DeviceMotionEvent.requestPermission()` to run inside a user gesture. The web
chassis originally handled that by injecting a global "Enable motion" button,
which made every Pax web app show gyro-specific UI on mobile.

Solved by keeping sensor startup in the web chassis but exposing
`window.paxRequestDeviceSensorPermissions()` for app-owned controls. The
`gyro-helper` example owns its visible `Enable motion` button and calls that hook
from its Pax `Button` handler, so other web apps do not inherit the prompt.

Recommendations: document the helper near `$gyro` / `$accel` web-target docs if
this becomes an officially supported developer-facing API, and consider a
typed Pax runtime API for requesting platform permissions instead of direct web
JS interop.

## 2026-05-21

Tried to use `examples/src/starter-project` as a quick iOS chassis smoke test,
but its path dependencies include other example crates with their own `#[pax]`
roots. Building it through `pax-cli` set `PAX_DIR` to `starter-project/.pax`,
then the dependent examples failed the active-project guard because their roots
did not match that `PAX_DIR`.

Solved by switching chassis validation to a single-root example such as
`examples/src/increment` or `examples/src/router-playground`.

Recommendations: document preferred single-root smoke-test examples for chassis
work, or add a dedicated minimal iOS/macOS validation example that avoids
cross-example Pax dependencies.

While converting `neon-opacity` gradients to `@gradient`, elements following
`opacity=1` rendered with default slate fills. The parser had interpreted the
newline-separated `f` in the next `fill` attribute as the frame unit, producing
`opacity=1f` and a bogus `ill` setting.

Solved by making number+unit literals grammar-atomic so units must be adjacent:
`10px`, `50%`, and `(expr)px` are valid; `10 px` is rejected.

Recommendations: keep unit suffixes whitespace-free in docs and examples, and
add parser regressions whenever a grammar shorthand can consume the first
character of a neighboring attribute.

## 2026-05-25

The `color-picker` 2D saturation/lightness field was authored as generated
`ImageSource::Data` pixels, which made hue reactivity harder to reason about
while testing gradient syntax. The same HSL-style field can be represented more
directly as two orthogonal gradient fills: a horizontal gray-to-selected-hue
gradient under a vertical white-to-transparent-to-black gradient.

Solved by replacing the generated palette image with two `@gradient` rectangles
and driving the hue stop from a computed `Color` property.

While validating this on the iPad simulator, percentage stops rendered too early
because the GPU shader compared device-pixel-scaled coordinates against
unscaled stop distances. On a 2x DPR screen, the 100% stop landed halfway across
the fill. The symptom looked like broken stop alpha, but the manifest and
runtime color data were correct.

The same example also exposed native iOS slider jitter: the Swift view applied
the last runtime value on every render pass while the user was tracking the
thumb, so the control could fight an in-progress drag. A follow-up diagnostic
showed that stock iOS liquid-glass `UISlider` interaction itself uses smoothed
motion that can look like momentum or a spring-loaded release. That behavior is
native, not a Pax echo bug. The bridge keeps the native slider as the
hit-tested control, avoids runtime value writes during active tracking, keeps
the local Swift element value in sync with values sent to Rust, and avoids
echoing native-origin value changes straight back to the same native control.

The slider also exposed a runtime binding trap: `bind:` parsed correctly as
`DoubleBinding`, but generated property descriptor code applied it through
`Property::replace_with`. That copied the bound property's current value into
the child property instead of preserving the source property handle, so native
slider interrupts updated only the slider-local `value`. Unconditional final
`DoubleBinding` settings now assign the property handle directly.

The macOS color picker exposed a separate native-overlay tangent: the app
template only emitted `Click` interrupts from a SwiftUI gesture, while the
palette was authored with `@mouse_down`, `@mouse_move`, and `@mouse_up`.
Native-overlay containers also need to pass through empty hit-test regions so
canvas events can reach the vector event layer below. The macOS template now
dispatches pointer events from the canvas `NSView`, and the shared Apple bridge
routes `MouseDown` / `MouseMove` / `MouseUp`.

Recommendations: prefer vector gradients over generated image buffers when a
visual is naturally expressible as fills; it keeps reactivity in the normal
property graph and gives gradient syntax examples more realistic coverage. When
debugging gradient alpha, check the renderer's coordinate space before assuming
parse or manifest loss. Native form controls should not overwrite active user
tracking state from stale runtime values. For form controls, verify that
`bind:` preserves property identity, not only initial value mirroring. When a
target supports both canvas and native controls, verify event parity for
`Click`, mouse down/move/up, wheel, and touch rather than assuming a click-only
smoke test covers drag-oriented widgets.

## 2026-05-28

Firefox websocket failures from the designtime privileged-agent connection can
arrive as generic browser events. `ewebsock` 0.4 read `ErrorEvent.message()`
through a generated wasm-bindgen getter, which could throw
`expected a string argument, found undefined` before Pax saw the close/error
event. The client then logged noisy partial failures instead of quietly keeping
the designtime session alive for reconnect.

Solved by upgrading `pax-designtime` to `ewebsock` 0.8, whose wasm error path
uses JS reflection for optional error fields, and by keeping explicit tests for
close/error events scheduling reconnect without failing `handle_recv`.
Direct wasm validation also required enabling the `web-sys` `Location` feature
because `DesigntimeManager` reads `window.location().origin()`.

Recommendations: when browser websocket behavior changes, validate
`pax-designtime` directly with `cargo check -p pax-designtime --target
wasm32-unknown-unknown` in addition to native unit tests.

## 2026-05-29

Clean Ubuntu first-touch validation exposed host-side Linux development
dependencies that are easy to miss from macOS. Building `pax-cli` from source on
Ubuntu 26.04 ARM64 failed in `glib-sys` until the VM had GLib, Cairo, and Pango
development packages installed. The dependency came through Pax's current
text/rendering stack, not through the generated user's app logic.

Solved in the first-touch Ubuntu harness by adding `libglib2.0-dev`,
`libcairo2-dev`, and `libpango1.0-dev` to the baseline package set.

The same smoke also showed a stale web-interface contract path. Non-libdev
projects receive the web interface from `pax-compiler`'s embedded
`files/interfaces/web/public` output. The TypeScript source had mostly moved
from `occlusionLayerId` to `renderLayerId`, but an ignored generated public
bundle and `photoPickerCreate` still carried the old name. A fresh generated app
compiled successfully, then failed at runtime with `undefined id or
occlusionLayer` and rendered a black page.

Recommendations: after changing compiler/runtime interface message shapes,
rebuild the web interface bundle before packaging or source-linked smoke tests,
and grep both `src` and generated `public` interface files for retired contract
terms. For first-touch CI, include a browser smoke that loads and clicks the
fresh generated app, not just `pax-cli build`.

Windows 11 25H2 Arm64 first-touch provisioning in Parallels exposed two
workstation-harness traps. First, the guest computer name is still constrained
to 15 characters, so a natural VM name like `pax-windows-first-touch` cannot be
used as the Windows `ComputerName`. Second, the current Microsoft Arm64 media
can still present the OOBE license page even with the documented
`HideEULAPage` unattended setting and matching registry value present.

Solved for the harness by separating prerequisite installation from OOBE
completion. The Parallels install still creates the local `pax` admin user, but
the host waits for Parallels Tools authentication and then uses `prlctl enter`
to create and run a highest-privilege scheduled task as that user. This lets the
baseline install Visual Studio Build Tools, Git, Rust, the WebAssembly target,
and `wasm-pack` while preserving the visible OOBE page as an interactive desktop
checkpoint rather than a provisioning blocker. The harness also removes the
default Parallels sound device because it can trigger a macOS microphone privacy
modal that blocks console interaction and is irrelevant to Pax CLI validation.

Recommendations: keep Windows first-touch automation explicit about the split
between CLI/workstation prerequisites and interactive OOBE completion. Use a
short default computer name such as `pax-win-touch`, avoid assuming unattended
OOBE flags suppress every 25H2 screen, and disable VM devices that create host
privacy prompts unless a test explicitly needs them.

## 2026-06-02

Clean macOS first-touch validation exposed a web-build prerequisite that was not
called out in the public setup instructions. `pax-cli` installed successfully
from local source, but `pax-cli build --target web` failed while building the
default web interface because `pax-compiler/files/interfaces/web/build-interface.sh`
shells out to `npm`.

Solved in the source-linked first-touch harness by adding Node.js/npm to the
workstation baseline and recording their versions in the prerequisite markers.
This should not become a normal user prerequisite: release packaging should
build `pax-interface-web.js` and `pax-interface-web.css` before publishing
`pax-compiler`, and release validation should assert those generated artifacts
are included in the crate package.

The same Windows baseline showed that Visual Studio's broad C++ workload can
still omit the native ARM64 linker on Windows Arm64. Rust's
`aarch64-pc-windows-msvc` toolchain then failed at `cargo install wasm-pack`
with `link.exe` missing even though Build Tools was present. Once the ARM64
linker was present, `ring` also required `clang` during the `wasm-pack`
install.

Solved by explicitly adding `Microsoft.VisualStudio.Component.VC.Tools.ARM64`
on ARM64 workstations plus `Microsoft.VisualStudio.Component.VC.Llvm.Clang`
and `Microsoft.VisualStudio.Component.VC.Llvm.ClangToolset`, and by checking
for a matching `VC\Tools\MSVC\*\bin\Hostarm64\arm64\link.exe` and `clang.exe`
before deciding that Build Tools is complete.

Windows source-linked smoke testing from a macOS host exposed a host archive
metadata leak. BSD tar can emit AppleDouble `._*` entries for extended
attributes, and Windows then extracts those binary metadata files into the
checkout. The `pax-docs` build script walks docs files and expects UTF-8, so the
first Windows `cargo install --path pax-cli` failed while trying to read
`._*.md` sidecar files, even though the real source files were valid.

Solved in the Windows smoke harness by setting `COPYFILE_DISABLE=1` for the
host tar step and excluding `._*` / `__MACOSX` entries, in addition to
dereferencing symlinks and excluding platform build directories.

Recommendations: any macOS-hosted tarball used as a Linux or Windows source
checkout should disable AppleDouble emission and exclude host metadata. Build
scripts that recursively read docs or templates should ignore known metadata
sidecars before attempting UTF-8 parsing.

## 2026-05-31

Tried to author a component that places the first few projected children in
fixed positions and then renders the rest together. Existing `slot(index)` only
addressed one child at a time, so examples had to mirror child count or invent
intermediate props just to express a natural composition shape.

Solved by adding `slot()` as a declarative remainder projection. The runtime
derives a component-local projection resolution from current projected children,
active slot sites, and explicit slot indices; no persistent drain cursor is
stored.

The example also exposed a separate layout ergonomics issue: a `Stacker` treats
the `Slot` projection site as one direct stackable child, even when `slot()`
projects multiple children. That means `slot()` can semantically return a
subtree, but it does not yet make each projected child its own Stacker cell.

The duplicate-slot panel exposed a related authoring trap: wrapping each
`slot(...)` site in its own `Stacker` made the example look like duplicate
explicit slots were allowed to render, because the layout wrapper also
participates in projection and can obscure the shared projection-resolution shape
the example is trying to demonstrate. Fixed-size visual buckets that should not
re-project their content should use `Group` wrappers instead.

Styling the example also hit the easy-to-miss Pax z-order rule: the first
template child receives the highest z-index. A full-screen background rectangle
placed before foreground content covered the entire scene in the web renderer.
The expanded-tree inspector made the issue obvious by showing the background
node above the title and panels.

Dynamic slot re-dealing exposed a native-overlay artifact: a projected child
whose visual identity included `Text` could move between slot homes while its
native text descendants lagged or disappeared. Canvas primitives moved
correctly. The example avoids that artifact by making the gem tiles pure canvas
rectangles and keeping explanatory text outside the projected child.

macOS dev-tool verification exposed a host lifecycle trap: launch-time session
registration can succeed before the SwiftUI `WindowGroup` has produced a
drawable canvas, so `pax-cli dev list` may briefly show a session that never
heartbeats or services filesystem requests. Treat a one-time registration as
insufficient proof that the macOS chassis is actually ticking; confirm a
follow-up heartbeat, a successful `pax-cli dev look`, or a processed request.
When the canvas moves between windows, do not permanently shut down the display
link on a transient `viewWillMove(toWindow: nil)`; defer shutdown and confirm
the view is still windowless.

The same macOS example exposed a retained-rendering edge case in scrolled
content. Slot re-dealing moved a canvas-only gem tile from one fixed bucket to
another, and the live expanded tree/ray-cast correctly showed the old bucket as
empty, but the old physical surface still displayed a stale gradient sheen.
Skipping a dirty node on the newly targeted surface is not enough when a node's
coverage moves across retained render surfaces; the old surface must also drop
its retained copy of that node.

Relaunching the macOS example also showed that a hidden or non-visible SwiftUI
window can keep the app process alive without giving dev tooling a ticking
window. The app delegate now preserves the normal launch activation path and
creates an explicit fallback window only when no visible, key-capable window
exists.

Recommendations: prefer `slot()` for "fixed children plus rest" component APIs,
keep remainder semantics documented near component composition docs, and use
`examples/src/slot-projection-resolver` when changing slot resolution,
control-flow slot ordering, or projection diagnostics. If we want "rest children as
individual Stacker cells," design that as a container/layout transparency
feature rather than coupling it to slot resolution. Use `Group`, not `Stacker`, as
the fixed-size wrapper for a slot site when the example is testing component-wide
slot resolution instead of Stacker projection behavior. When a screenshot shows
only a background layer, check `pax-cli dev inspect tree` before chasing layout
math. When validating slot reparenting across native elements, include explicit
native-overlay checks; canvas-only examples are not enough to prove native
descendant reparenting. For renderer artifacts, compare framebuffer screenshots
against `ray-cast` or `inspect tree`: if the live tree is correct and only the
framebuffer is stale, audit retained renderer cleanup across every physical
surface the node used to intersect, not only the currently targeted replay
surface.

## 2026-06-03

The `materials` example exposed a native-cartridge gap that host-side
`cargo check` did not catch. New public template-visible API types such as
`Material`, `Depth`, `LightShape`, and `Vector3` compiled in the source crate,
but the generated iPadOS crate calls `<Type>::register_all_functions()` for
template-exposed value types. Without at least an empty `HelperFunctions` impl,
the native target failed during generated crate compilation.

Solved by adding `HelperFunctions` impls for the new lighting/material value
types and registering the intended `Material::*` and `Vector3::new` helpers.

The same simulator run caught a WGSL reserved-word issue that ordinary Rust
checks cannot see. A uniform field named `meta` parsed on the Rust side but
failed when wgpu created the shader module on iPadOS because `meta` is reserved
by WGSL/Naga.

Recommendations: when adding template-visible value types, test at least one
generated native cartridge path, not only the source crate. When changing WGSL,
prefer boring names such as `flags`, `params`, or `state`, and use an actual
wgpu runtime launch to validate shader parsing on the target backend.

The source panel in `slot-projection-resolver` exposed a web chassis regression:
a native-only `Scroller` could still receive a scroller-owned WebGPU canvas
layer. Because the layer had no vector drawables, it displayed only the browser
surface clear color, so the native text pane appeared to have an unexpected
opaque backing even though the DOM/native text subtree was transparent.

Solved by publishing the set of render layers that actually contain canvas
drawables from the engine occlusion pass, then returning an inactive canvas plan
for scroller-owned web layers that have no canvas work. The native scroller host
still exists and can clip/scroll native content, but the chassis no longer
materializes an empty GPU surface behind it.

The same example also showed the canvas-backed half of the issue: a horizontal
tab scroller with vector children still needed a scroller canvas, but empty
pixels in that canvas cleared white. The browser WebGPU backend in wgpu 28
reports only `Opaque` surface alpha in capabilities even though its configure
path accepts `PreMultiplied` and maps it to browser premultiplied canvas alpha.
The web chassis now requests premultiplied alpha for browser WebGPU surfaces so
canvas-backed scroller islands can clear transparent instead of exposing the
opaque fallback color.

Recommendations: when a transparent native-backed scroller looks opaque on web,
inspect both DOM backgrounds and the scroller's canvas plan before adding app
backing rectangles. Main scrollers with vector descendants can mask this bug
because their canvas content covers the empty clear; native-only scrollers are
the cleaner regression test. Also include at least one canvas-backed scroller
with empty pixels in transparent-scroller validation, since it exercises browser
surface alpha rather than only empty-plan suppression.

## 2026-06-04

While tuning the `materials` example, a browser refresh reloaded the baked web
cartridge instead of the latest hot-reloaded `.pax` state. The source had moved
the charcoal root underlay below the scroller, but `.pax/cartridge.partial.rs`
and the served wasm still contained the earlier child order until the web target
was rebuilt. Touching the `.pax` file caused a file event but did not bring the
refreshed connection up to the latest template state.

The same pass exposed a browser scroller-layer compositing trap: the root
underlay is the right visual model, but opaque browser canvas surfaces clear to
white, so transparent gutters inside a browser-owned scroller showed white
instead of the root charcoal.

Solved for the example by rebuilding after the `.pax` edit and by adding an
unlit charcoal rectangle inside the scroller content as the gutter backing,
while keeping the root charcoal underlay in place.

Recommendations: refreshed designtime connections should catch up to the latest
template state instead of only loading the baked cartridge. When examples rely
on root backgrounds showing through browser-owned scroller layers, verify the
browser target specifically; opaque canvas fallbacks may need an in-layer
background or a renderer-level alpha/compositing fix.

## 2026-07-07

While validating path drawing progress with a web example, native `cargo check`
passed but the generated web cartridge first failed because the new
template-visible `UnitValue` API type did not implement `HelperFunctions`. Once
that was fixed, the browser target exposed a separate renderer panic:
multi-contour paths converted each `MoveTo` to a lyon `begin()` without ending
the previous subpath, triggering `multiple begin() calls without end()`.

Solved by adding the no-op `HelperFunctions` impl for `UnitValue` and by ending
an open lyon subpath before beginning another in the kurbo-to-lyon converter.
The `path-drawing` example now includes multiple closed contours to keep
this case exercised.

Recommendations: when adding template-visible value types, run at least one
generated target build, not only the source crate. SVG- and Illustrator-derived
path work should include multi-contour browser validation because closed
outline text naturally produces many subpaths.

The same example exposed a path-trim fidelity issue that was invisible in
native checks: trimming via flattened `PathSeg`s lost `ClosePath` metadata, so
completed earlier contours in a multi-contour path were stroked as open paths
until the entire path reached 100%. In outline text this showed up as corners
and joins that stayed capped and then snapped closed at the very end.

Solved by tracking contour metadata during trim and emitting `ClosePath` as
soon as a closed contour is fully visible, including the zero-length close case
that occurs when an SVG has already drawn an explicit return-to-start segment.
Keep visual browser validation for path drawing examples; stroke joins and caps
can look wrong even when the path length math and native tests pass.

Adding the `Handwriter` test-bed row exposed a string authoring wrinkle:
`text="line one\nline two"` arrived at Rust as a literal backslash-plus-`n`
rather than a newline, so the generated stroke font drew the escape sequence.
For this component, `Handwriter` treats `\n` as a line break during path
generation.

Recommendations: clarify string escape semantics in the template-language docs
or add parser support/tests for common escapes. Component-local tolerance is a
reasonable stopgap for user-facing text fields, but language-level behavior
should be explicit.

The Handwriter row also made stroke joins visible in a way ordinary examples
did not: the SVG stroke font's polylines had acute turns, and the renderer's
implicit miter joins produced small spikes at cursive letter corners. The
translucent guide path made those spots look darker because the same angular
geometry was alpha-composited twice.

Solved by adding explicit `StrokeJoin` support and by using round joins for
`Handwriter`. Keep miter as the default for compatibility, but use
`StrokeJoin::Round` for handwriting, sketch, and pen-like paths.

Scaling the EMSTech `Handwriter` row made another glyph-data artifact obvious:
some bundled SVG stroke fonts encode curves as dense polylines, so larger
strokes reveal the original segmentation even when joins are rounded.

An attempted `Handwriter.smoothing` pass improved visual quality by converting
line chains into cubic curves, but it made draw-range animation performance
unacceptable. A follow-up per-`PathInstance` trim-analysis cache reduced only
one CPU-side layer and left the larger per-frame tessellation cost intact, so
both changes were reverted.

Recommendations: revisit handwriting smoothing only alongside a renderer-level
path-drawing strategy that preserves full stroked geometry and varies the
visible range without changing submitted path geometry every frame.

## 2026-07-21

The canonical Pax logo exported by Illustrator encoded its post as a
`<polygon>`, while the SVG importer supported only `<path>`. Import succeeded
with a warning but ejected an incomplete logo, which made the omission easy to
mistake for an animation or layout bug.

Solved by importing polygons as closed Pax `Path` nodes in document order,
using the same inherited style and transform pipeline as SVG paths. Polygon
point parsing now rejects malformed, odd-length, and fewer-than-three-point
inputs, and CLI validation reports path, polygon, and generated-node counts
separately. Recommendations: unsupported visible SVG geometry should be made
conspicuous in validation output, and every newly supported element should
share style, transform, ordering, and malformed-input coverage rather than
growing a parallel importer path.

The first animated pass also exposed two geometry canonicalization gaps in the
same source: the sign path contained a zero-length `h0`, and the post polygon
repeated its first vertex immediately before the implicit close. Both became
degenerate Pax line segments, making invalid imported geometry an additional
variable during animation debugging. The importer now drops line
segments whose transformed, quantized endpoints are identical, removes
consecutive polygon duplicates, and removes a polygon's redundant final copy
of its first vertex. Animation keyframes also use direct percent literals; this
avoids turning percent-only positions into `Size::Combined` through
`$base +/- percent` arithmetic. Recommendations: canonicalize imported geometry
at the precision actually emitted, and prefer literal values for fixed timeline
geometry when relative expressions add no reuse value.

The logo's selector-targeted named timeline initially declared the same `x`
and `y` properties inline on its motion groups. Named timelines do not override
an inline setting, so those tracks were omitted from the generated node
settings: the playhead advanced while the intended translations stayed at
their static values, which looked like abrupt entrances rather than a broken
timeline. Solved by leaving timeline-owned properties off the element and
using the last keyframe as the resting value. Replay keeps the paths mounted
until the keyed component instance is replaced; mounting the replacement
restarts its `@in` timeline without an application-owned clock. Recommendations:
treat each animated property as single-owner data, use keyed remounting when a
self-contained entrance animation must replay, and inspect the expanded node
transform at a frozen intermediate playhead before tuning choreography.

Animating the same logo also exposed that a `Mask` produced an empty clipped
result for both a nested custom component and direct `Path` descendants, even
though the expanded tree contained the paths and the board rendered correctly
without the mask. A width-animated `Frame` around the direct paths likewise
failed to paint the board. Combining ancestor opacity with a large horizontal
scale also made the filled compound paths remain absent during the transition
and appear only when the transform settled.

The example was kept reliable by splitting the ejected geometry into explicit
board and post components, then animating the full-size, fully opaque board as
a rigid assembly from off-canvas. The post occludes its leading edge so the
board appears to unfurl from the attachment point without a mask or degenerate
scale. Recommendations: add focused `Mask` and animated-`Frame` regressions for
direct and component-backed path subtrees, and verify that ancestor opacity and
transform changes dirty filled canvas descendants on every frame.

## 2026-07-08

While exploring a complex filled-SVG stress fixture for PAX-967, an obvious
public-domain-looking reference asset turned out to be AGPL-licensed, which is
a poor fit for checking into Pax examples. Later alternatives also exposed that
complex generated Pax source can become large quickly, and the fill-reveal
experiment was not strong enough to justify keeping the fixture in tree.

Recommendations: verify SVG fixture licenses before import, avoid checking in
large generated Pax fixtures when the original SVG can remain the source asset,
and keep complex fill-animation experiments out of narrow stroke-drawing
examples until their semantics are compelling.

While adding a reusable `FontComparisonRow` component to the path-drawing
example, the component rendered inside fixed-height stacker rows but clipped
and aligned inconsistently until each call site specified `width=100%` and
`height=100%`.

Solved in the example by treating reusable row components like ordinary layout
children: the containing `Group` owns the row size, and the component instance
explicitly fills that row before its internal scroller uses percent sizing.

Recommendations: when extracting a repeated Pax layout into a component, keep
the outer component dimensions explicit at the call site if the component's
template relies on percent-sized descendants. If this keeps surfacing, consider
improving docs or defaults around component-root sizing in layout containers.

The same path-drawing example exposed a likely web scroller fidelity issue:
stroke edges in large `Handwriter` paths looked aliased on a DPR 2 display even
though `Handwriter` emits vector `PathElement`s and `Path` strokes them at the
resolved pixel bounds. Browser inspection showed the root visible canvas backed
at 2x, but additional scroller-related canvases were backed at 1 canvas pixel
per CSS pixel.

Recommendations: treat jagged large vector strokes inside web scrollers as a
potential surface backing-scale issue before blaming the primitive. A focused
regression should inspect all visible layer/tile canvases and verify that
scroller-owned vector surfaces honor device DPR unless deliberately clamped.

The static curve-fitted `Handwriter` font comparison rows caused a severe web
performance cliff when they shared a retained vector surface with active
`draw_start`/`draw_end` animations. Browser sampling showed the renderer main
thread saturated in Wasm/JS. A/B measurements isolated the cost: removing the
comparison rows recovered the example, one original-plus-curved row was still
slow, one original-plus-original row was near baseline, and keeping a curved row
while pinning all draw ranges static was smooth.

Solved for the example by dropping the bundled curve-fitted font variants and
removing the comparison grid. Keep curve fitting as an offline experiment until
path drawing can vary visible stroke range without rebuilding or restroking
heavy cubic glyph geometry on every animated frame.

After render-side path trimming landed, the same smoothing idea became viable
again with a different boundary: smoothing must be a retained geometry input,
not a per-frame `PathInstance::render` preprocessing step. `PathSmoothing`
now rides with the vector op into the WGPU renderer, participates in the
geometry signature, and runs immediately before tessellation. Animated
`draw_start` / `draw_end` changes remain primitive updates, so smoothed
handwriting does not resmooth or retessellate on each timeline tick.
Profiling the macOS chassis then exposed a second boundary: the occlusion pass
must not rebuild stroked coverage for every draw-range tick either. `Path`
occlusion should use a conservative full-footprint path by default, with
progress-sensitive native masking deferred until there is a concrete need.

`Handwriter` also exposed the need for semantic text behind vector handwriting.
The first pragmatic solution is to keep the visible strokes as `Path` geometry
while layering an invisible native `Text` node using either `alt_text` or the
drawn `text`. This is not a complete cross-primitive accessibility system, but
it gives screen readers, crawlers, and selection machinery a real text object
for the handwriting case.

That semantic `Text` layer is only an approximation for mouse selection. The
visible `Handwriter` path normalizes stroke-font geometry to fill its component
bounds, while native text preserves normal font metrics, so a default 20px
top-left text layer produces visibly detached selection rectangles. The current
local fix centers the invisible text and sizes it by line count so selection is
roughly in the right region. True "select the handwritten strokes" behavior
would need a deeper text/native bridge, such as bounds-aware native text scaling
or a platform accessibility/selection overlay that can use the rendered vector
bounds instead of ordinary font metrics.

## 2026-07-13

Restarting or losing the design server during a `path-drawing` Rust rebuild
exposed a split-generation failure: the still-running web cartridge had a
static component descriptor registry, but it could install the newly built
server manifest before JavaScript mounted the matching JS/Wasm artifact. A
missing `FontComparisonRow` then caused a runtime panic, and subsequent calls
reported wasm-bindgen recursive mutable-borrow errors because the first panic
had crossed the Wasm boundary.

Solved by routing Pax-only edits and application-logic replacements through a
shared debug revision coordinator, checking incoming manifests against the
executing cartridge's component/type ABI, and publishing each web reload
directory atomically only after its required JS, Wasm, and wasm-bindgen
snippets tree exist. Web and macOS now prepare and commit compiled artifacts
through the same two-phase revision transaction; iOS participates only in the
Pax-template lane. The protocol also names a separate interpreted-module mode
without treating a future JavaScript evaluator as a dynamic library.

Recommendations: whenever static generated code and dynamic program metadata
travel through different channels, make their shared generation explicit and
validate compatibility before mutation. Treat a panic followed by repeated
wasm-bindgen aliasing diagnostics as one poisoned boundary until proven
otherwise; the first panic is usually the useful error.

The same session showed that image loading retried non-success HTTP responses
but not fetch, blob-decode, bitmap-decode, or canvas exceptions. Its async
microtask also surfaced failures as unhandled promise rejections and could
finish after its Wasm cartridge had been disposed.

Solved by retrying the complete decode pipeline, catching the host-side promise,
and checking native-pool ownership before delivering decoded pixels to Wasm.

Recommendations: keep fallible browser I/O outside Wasm mutation boundaries,
attach resource paths to the final diagnostic, and re-check object generation
after every await before calling into a disposable runtime instance.

The server-stop browser drill also exposed a fixed 500ms websocket reconnect
loop. The UI kept animating, but each failed attempt emitted both a low-level
browser error and a runtime warning. Capped exponential backoff preserves quick
first recovery while avoiding sustained console and connection churn during a
longer outage.

The final web drill exposed a separate transport-state race: constructing a
websocket marked it alive before the browser emitted `Opened`, so a revision
message could reach `WebSocket.send` while the socket was still `CONNECTING`.
Solved by treating sockets as offline until `Opened`; the revision gate remains
the durable owner of messages that must replay. Recommendations: distinguish
object construction from transport readiness, and never use socket existence
as evidence that writes are legal.

Manual Rust reloads on web and macOS also showed duplicate watcher
notifications for one source save, which can schedule two sequential normal
artifact builds. The same save sequence exposed editor backups such as
`lib.pax~`, while a global one-second "ignore our own writes" window could drop
an unrelated external save entirely.

Solved by filtering unsupported and transient paths before reading, remembering
the last contents observed for each canonical source path, and suppressing only
an exact path/content echo registered by a server-authored write. The logic
worker now distinguishes its debounce window from compilation: events before
the build snapshot collapse into that build, while a changed source observed
during compilation still queues one follow-up. Recommendations: source-watcher
deduplication should be resource- and content-specific; global time windows
hide real edits, and marking a worker busy before a debounce can accidentally
turn duplicate events into duplicate builds.

Refactoring reload generations exposed two less obvious coupling points. The
source watcher delivered changes through the currently active websocket actor,
so edits made during a disconnect were never committed to server state. Apple
mobile debug runs also built designtime support into the cartridge without
provisioning or injecting a server, leaving that path effectively untestable.

Solved by making the debug revision coordinator, rather than a socket actor,
the owner of source changes, and by having iOS/iPadOS debug launches provision
an explicitly template-only server. Recommendations: treat connectivity as a
delivery concern rather than the owner of mutable program state, and keep
template transport, compiled-artifact replacement, and future interpreted
logic activation as separately declared capabilities for every chassis.

Watcher and authoring edits initially still had a stale-buffer race: parsing a
file happens outside the coordinator lock, so a slow watcher callback could
commit an older disk buffer over a newer authoring write. A single global
revision check was also too coarse because independent source files should be
able to compose rather than continuously supersede one another.

Solved by putting source mutation and active-client promotion behind one
transaction barrier, assigning a mutation generation to each canonical source
path, and comparing that generation plus the full revision stamp at commit.
Recommendations: when parsing or code generation occurs outside a state lock,
make the commit token specific enough to reject same-resource staleness without
discarding valid concurrent work on unrelated resources.

The first replay implementation treated that journal as a lifetime patch set
for every future logic artifact. That preserved edits made during a build, but
it also meant an old entry for a source file deleted or renamed by a later Rust
revision could make every subsequent activation fail permanently. Each logic
build now captures the journal generation immediately before compiling, and
preflight replays only mutations newer than that build-start baseline.
Recommendations: replay logs that bridge a build race need an explicit snapshot
boundary; "latest per resource" is not enough when resources can legitimately
disappear from later generations.

A process restart exposed one more distinction: reconnecting a still-running
cartridge is not safe if the new server invents a fresh revision identity or
forgets which compiled artifact was committed. A manifest alone is also
insufficient because a fresh web page may need the retained artifact envelope,
and edits can land on disk after the last successful publish.

Solved with an atomically published debug restart record containing the exact
revision stamp, active manifest, retained artifact envelope, and survivor
adoption authority. Startup restores that record, reconciles active `.pax`
sources from disk, and schedules one recovery logic build on web/macOS. A new
logic revision is not committed if the record cannot be published; an already
committed duplicate activation remains idempotent. Recommendations: treat
durable process-restart state as the commit record for a live-reload
transaction, keep it out of release cartridge baking, and explicitly define
which source of truth wins when persistence fails after an external editor or
an authoring client has already changed the file.

The first macOS chassis drill also exposed a LaunchServices boundary that is
easy to miss: setting `PAX_DEV_SESSION_DIR`, `PAX_DEV_REGISTRY_FILE`, and the
design-server address on the `open` helper process did not put them in the app
bundle's environment. The renderer still opened, which made the detached dev
session look like a websocket or heartbeat problem.

Solved by passing every dev variable through explicit `open --env NAME=VALUE`
arguments. Recommendations: when a macOS app is launched through
LaunchServices, verify the environment in the final app PID and require a
follow-up heartbeat or serviced dev request; a live renderer alone does not
prove that the designtime transport was provisioned.

The web interface package's nominal `npm run build` command currently points at
a nonexistent `webpack.prod.js`, even though the supported bundle path is the
repository's `build-interface.sh` script (which invokes esbuild and emits the
gitignored public JS/CSS artifacts). Use `bash build-interface.sh` when
validating host TypeScript changes; a webpack configuration error does not
indicate a TypeScript or Pax runtime failure.

## 2026-07-18

The iOS chassis renders Pax edge-to-edge, so a mobile header authored at
viewport `y=0` can paint beneath the system status area while UIKit intercepts
touches there. In `router-playground`, this made nearly all of a visible 44px
Menu button unhittable even though the Pax hit bounds matched its fill.

Solved locally by deriving an iOS-only top inset from `NodeContext::os` and
using the same resulting header height for the controls, drawer, underlay, and
content outlet. Mobile web retains its original header geometry.

Recommendations: expose chassis-provided safe-area insets through runtime
viewport data. Until then, edge-to-edge examples with top-level controls must
explicitly reserve the native status region, and visual bounds alone are not a
reliable test of iOS hit accessibility there.

## 2026-07-20

Wrapping canvas-interactive content in a `Scroller` changed input ownership on
iOS: the native `UIScrollView` correctly received pan gestures, but it sat above
the Pax canvas and prevented taps from reaching descendant `@click` handlers.
The macOS Scroller path already forwarded pointer interrupts explicitly; the
iOS path had no equivalent.

Solved by adding a non-cancelling tap recognizer to the iOS Scroller host. UIKit
first distinguishes a tap from the scroll view's pan gesture, then the host
forwards the settled viewport coordinate to Pax hit testing. Nested native
controls and nested scroll views retain their own gesture ownership.

Recommendations: any native host layered above canvas-interactive descendants
must explicitly preserve their semantic input path. For scrolling surfaces,
forward taps only after native gesture arbitration rather than forwarding raw
touch-down events, which would activate content when the user intends to drag.

## 2026-07-21

Changing a nested route inside an autosized iOS `Scroller` reset the surviving
outer scroll position. Native-tree reconciliation replayed the Scroller
element's cached offset for every structural patch, even when that patch only
changed content geometry. During route transitions, a temporary content-size
contraction could also make UIKit clamp `contentOffset` and report the clamp as
if it were a user scroll.

Solved by applying cached scroll state only when the native host is created and
delivering later explicit scroll patches directly to that host. iOS content-size
updates now preserve the host's logical position while suppressing synthetic
delegate callbacks, allowing the position to return when autosized content
expands again. Recommendations: treat scroll offsets as patch-owned state;
structural reconciliation must not replay them, and platform layout clamps must
not silently become user-authored state.

Trying to replace per-gem sheen overlays with a scene light in the
`slot-projection-resolver` example exposed that light membership is currently
layer-global. One gem light also changed panels, controls, outlines, scroller
backings, and other default-lit vector chrome. Preserving the original scene
required defensive `Material::unlit()` settings on every unrelated primitive;
an omitted opt-out could change UI far from the component that authored the
effect. A local light could also dim inaccessible siblings through the default
ambient term even if its direct contribution were filtered later.

The PAX-966 MVP design introduces a lexical `LightFrame`: a light belongs to its
nearest frame, outer lights may enter nested frames, and inner lights cannot
escape to parents or siblings. Membership follows expanded render-parent
ancestry, so repeated components and projected slot content behave according to
their mounted visual tree. Primitives with no eligible light and no authored
ambient retain identity lighting rather than receiving the default lit-scene
ambient.

Recommendations: prove scoped lighting with both a repeated hover-light case
and the gem-tile regression; keep `AmbientLight` layer-wide for the first slice;
and treat arbitrary selector targeting and component-authored singleton light
resources as separate follow-up designs.

## 2026-07-22

A modal underlay authored as a translucent canvas `Rectangle` did not composite
above canvas content hosted by a native `Scroller`. The root canvas contains the
underlay, while a Scroller owns a separate CAMetalLayer-backed canvas island in
the native scene. Treating full-surface native-mask coverage as uniform island
alpha attenuation was not equivalent to painting translucent black over the
finished scene and left light card fills visibly brighter than surrounding
content.

Solved by allowing `EventBlocker` to paint an optional solid background and
using that native, z-ordered surface for the `RouteCard` and `RouteModal`
underlays. The underlay now blocks input and composites above retained native
and scroller surfaces; opaque incoming canvas content masks it through the
normal occlusion path. Recommendations: use a real surface in the native z
stack for translucent modal underlays that span canvas islands. Do not model
source-over color compositing by attenuating the covered surface's alpha.

Building the PAX-966 `GlowButton` example exposed three adjacent integration
boundaries. First, mouse and touch payload documentation described coordinates
as node-local even though the web and native chassis send window coordinates.
The existing `NodeContext::local_point` conversion is the right transform-aware
path, so the payload docs and event guide now state that contract explicitly.

Second, touch move and end events were re-hit-tested at their current position.
A direct-manipulation effect therefore stopped receiving updates as soon as the
finger left its original bounds, and could remain visually pressed. The runtime
now captures the topmost hit node per touch identifier at start, routes move,
end, and cancel to that captured subtree, and releases capture at completion,
cancellation, or node removal.
Recommendation: keep pointer ownership in shared runtime dispatch rather than
reimplementing capture independently in each chassis or component.

Putting that example inside a native iOS `Scroller` exposed a related input-
island boundary: the `UIScrollView` sits above the Pax canvas, so the canvas's
touch overrides never see contacts that begin inside the Scroller. The native
Scroller must forward touch start/move/end in Pax viewport coordinates while
letting UIKit's pan recognizer arbitrate.

Forwarding from `UIScrollView.touchesBegan` was too late: UIKit delays delivery
to content while deciding whether a vertical pan will win, which made immediate
feedback appear only for quick taps, held contacts, and motion outside the
enabled scroll axis. Cancelling the captured child from
`scrollViewWillBeginDragging` compounded the problem by conflating scroll
ownership with touch-stream cancellation. Observe contacts with a non-cancelling
gesture recognizer that recognizes simultaneously with the native pan instead.
The child receives start, move, and end even while scrolling; the pan only
disqualifies tap activation. Reserve `TouchCancel` for an actually aborted
contact, and keep tap recognition movement-aware on every chassis so a completed
drag cannot synthesize a click.

Third, Rust compilation did not validate the embedded WGSL shader. A helper
scope error compiled successfully and failed only during WGPU pipeline creation,
which surfaces as a fatal native validation crash. A focused Naga test now
parses and validates the shipping geometry shader, alongside a Rust/WGSL storage
layout assertion. Recommendation: every shader or GPU data-layout change should
run host-side shader validation before launching an example.

The first web showcase build also encountered a workstation-specific toolchain
boundary: the active Node 14 runtime was below esbuild's supported range and the
global npm staging lock was owned by another user. The macOS build remained a
valid visual test chassis, but web validation should preflight the bundled Node
runtime and a writable npm cache before entering the Pax build.

Touch-driven local coordinates exposed one more Scroller boundary on web:
hit-testing already folded the browser-owned presentation scroll offset into
its event ray, but `NodeContext::local_point` inverted only the node's
content-coordinate layout transform. A touch on a vertically scrolled child
therefore produced a local Y displaced by the scroll amount (usually clamped to
the child's top edge), while X appeared correct when the Scroller had not moved
horizontally.

Solved by resolving the same accumulated ancestor-scroller presentation
transform for local coordinate conversion that hit-testing uses.
Recommendation: any API converting window coordinates into node-local space
must account for presentation-only transforms such as native/browser scrolling,
not merely the engine's layout transform.

Fading the final scoped light to zero exposed a separate lighting-state seam.
While the zero-intensity light remained enabled, its eligible material still
received the default 35% ambient term; disabling that final light then restored
identity rendering in one frame. The result looked like a dark pause followed
by a sudden flash back to the resting fill.

The `GlowButton` now eases its material ambient response toward the reciprocal
of the default ambient intensity while the direct light fades. At the
zero-light endpoint the lit result already equals identity rendering, making
the eventual light disable visually continuous. Recommendation: when an
authored lighting scene transitions back to identity, animate either scene
ambient or eligible material response to an identity-equivalent endpoint before
removing the final light.

While animating the Pax logo, a very small nonzero rotation on a `Group`
containing filled native `Path` descendants made those paths disappear on the
web chassis even though scene inspection reported a valid, on-screen expanded
transform. Translation-only tracks rendered every fixed playhead reliably, so
the first production pass avoids ancestor rotation rather than hiding the
failure with opacity or duplicated geometry. Recommendations: regression-test
filled paths under animated ancestor rotation before using rotation for
secondary action in reusable vector artwork.

The same investigation exposed a hot-reload boundary: edits inside a named
`@timeline` were detected, but the running web cartridge retained the previous
timeline definition while ordinary template edits did update. A full
`pax-cli run` restart regenerated the timeline and made the corrected values
active. The watcher update currently parses and transmits only the component
template and settings block; it neither carries `ComponentDefinition.timelines`
nor rebuilds the mounted component's merged timeline property layers.

Solved by carrying the parsed timeline definitions in
`UpdateTemplateRequest`, committing them into the active revision, and replacing
them through the designtime ORM. The existing tree reload then reconstructs
component and element property layers from the updated manifest. This required
no release-cartridge schema change because timelines were already represented
in the program IR, binary, and Rust-manifest paths. A live web test confirmed
that changing and restoring a named timeline keyframe updates the mounted app
without restarting `pax-cli`.

Frame-sequence diagnostics also had two capture traps. `pax-cli dev look`
returned black web-canvas frames in this example, while browser screenshots
used changed-region optimization that could make a valid animation look like
detached fragments. A temporary fixed-step playhead plus a full-screen,
playhead-driven background change produced reliable full frames and isolated
the rendering failure. Sequential browser screenshots added another source of
drift because image encoding time advances the live timeline between frames;
restarting the animation before every sampled timestamp produced trustworthy
contact sheets. Recommendations: provide an official fixed-playhead capture
mode for declarative timelines and an option to force full canvas frames in
animation tooling.

Refining the logo post from a translated rigid shape into an unfurling fabric
strip required animating `PathElement` geometry procedurally. Rebuilding the
same cubic-path topology from a shared playhead rendered reliably; changing
only control points and endpoints also made exact contact-sheet frames easy to
compare. The lower-edge overshoot exposed that `Path` was implicitly clipping
its rendering to its layout rectangle, even though Pax otherwise reserves
clipping for explicit containers and masks. Removing that clip also required
using the actual path geometry for retained-renderer tile coverage; otherwise
large overflow could still disappear when its layout bounds missed a tile.
Recommendations: keep procedural animation topology stable, let `Path` geometry
draw outside its layout bounds, and use `Frame` or `Mask` when clipping is part
of the authored result.

Moving that post animation from an imperative Rust playhead to `@timeline`
worked without interpolating `Vec<PathElement>` directly. The timeline owns a
small set of scalar motion controls (extension, wave, swing, and spool radius),
while one reactive Rust computation maps their instantaneous values
into stable-topology paths. This keeps timing, easing, and choreography visible
in Pax while retaining path assembly and screen-space geometry math in a pure
adapter. A sampled horizontal `whip` control was eventually removed in favor of
a small overshoot and return on `extension`; driving the existing physical
dimension directly made the bottom elasticity simpler and removed the
shockwave-like secondary deformation. Recommendations: expose artist-facing
scalar controls to timelines before adding collection interpolation or custom
expression helpers for procedural geometry, and prefer overshooting an existing
physical control over introducing a second deformation channel for follow-through.

An experiment that drove a long fabric edge with one cubic segment made impact
feedback read as a rigid sheet: lowering the temporal period only made the
entire silhouette convulse faster. Spatially phased upper, middle, and hem
anchors produced a more recognizable traveling wave, but repeated letter-impact
feedback still competed with the logo's primary choreography and was removed
from the final composition. Recommendations: model traveling deformation with
multiple spatial control bands rather than one global amplitude, and be willing
to remove a physically motivated secondary action when it weakens staging.

Lengthening the early part of a masked letter roll did not make its motion look
slower because the extra frames were still hidden behind the neighboring
letter. The visible portion remained compressed near the end and read as a
sudden appearance. The fix was to make arrival at the mask boundary a
first-class timing event, move the stone to that boundary early, and spend most
of the deliberately slow acceleration after it becomes visible.
Recommendations: tune masked motion in visible space rather than only in
playhead or property space, and keep translation, rotation, scale, and mask
handoffs phase-locked.

Adding a planted braking flourish to those procedural letter paths required a
second transform pivot after the main roll had reached its authored geometry.
Reusing the center-pivoted roll rotation made the whole glyph drift, while
editing the path points directly made the final logo harder to verify. The
working pattern keeps a separate timeline scalar for the follow-through angle,
then applies it as a pure post-transform around the glyph's lower-right contact
point in the path adapter. A piecewise recovery initially snapped at its easing
boundaries; chaining `InOutQuad` segments and holds at zero-velocity endpoints
made the hesitation, counter-rock, and settle C1-continuous without requiring a
custom curve. Recommendations: model anticipation and follow-through as
composable transform stages with explicit pivots, leave source geometry and its
final zero-state unchanged, and match endpoint velocities when assembling a
motion curve from serializable easing segments.

The declarative timeline accepts every fixed `EasingCurve` variant, but the
current enum contains only linear/hold, quadratic, and back families. Its
`Custom` variant stores a Rust closure, so it cannot be named or serialized by
Pax syntax; preserving an earlier cubic settle therefore required several
sampled linear segments. Recommendations: if authored motion needs arbitrary
curves, add a serializable cubic-Bezier easing value rather than trying to
expose closure-backed `Custom` easing through the manifest.

Building the unraveling spool as two circular contours overlapping a rectangular
contour inside one filled `Path` produced visible seams and detached-looking
fragments on the web chassis. The same geometry rendered cleanly when each cap
and the body were emitted as three independently filled paths. Recommendations:
do not assume overlapping closed contours within one native `Path` union their
fills; use separate filled paths when an animated compound silhouette depends
on overlap, until compound-path fill semantics are explicit and regression-tested.

The near spool cap was also clipped when its center traveled directly along the
post component's left boundary, exposing the rectangular roll body as an angular
outer edge. This was the same implicit `Path` clip rather than a component-layout
constraint. Recommendations: do not pad or rescale authored geometry merely to
work around primitive overflow; preserve source coordinates and apply an
explicit clipping primitive only when overflow should be hidden.

Adding a new custom component while a web `pax-cli run` session was active, then
starting a standalone `pax-cli build` for the same example, allowed both compiler
processes to mutate the example's shared `.pax` interface directory. The running
compiler failed while copying interface files and had to be restarted. The clean
build and a fresh run session succeeded. Recommendations: do not run concurrent
Pax compiler processes against the same project worktree; stop or reuse the
active run session before starting a standalone build.

Overlaying static logo letter paths on the animated sail exposed two geometry
translation traps. The post artwork occupies 96% of the component's presentation
height, so applying the source's Y percentages directly stretched the letters by
about 4.17% and broke their optical centering; the overlay needs the same 0.96
source-to-presentation Y mapping as the animated post. Separately, reversing a
cubic counter contour requires reversing segment order *and* swapping each
segment's two control points. Reversing only the endpoints turned one quadrant
of the `a` counter into a triangular wedge.
Recommendations: keep source-space-to-animation-space scaling explicit for
every static overlay, and unit- or visually test reversed cubic contours at
their cardinal points.

Using a fill-colored `Rectangle` as a temporary reveal mask for the logo's
animated `x` exposed a coordinate-space mismatch: the rectangle's element-frame
positioning shifted the cover left of the source-SVG boundary, leaving the `x`
visible behind the `a` while painting an unrelated black band over the `p`.
Replacing it with a real `Mask` whose source is a Pax-native path in the logo's
coordinate space made the reveal boundary exact and background-independent.
An axis-aligned half-plane was still insufficient once the neighboring `a`
gained a planted braking rotation: the `x` leaked past the tilted sidebar even
though the boundary tracked its world-space maximum X. Both an oversized
rotating half-plane and a canvas-bounded trapezoid disappeared on web during
the tilt, then returned when the mask approached axis alignment. The reliable
solution needed no synthetic mask: render `x` behind the complete `a`, letting
the actual white body and black counter occlude it as it rolls clear.
The `a` counter did not need a mask because its counter and outer paths already
share one computed transform; wrapping those paths in an extra mask caused the
counter to disappear on web. Recommendations: use a true mask for cross-boundary
reveals, express its coverage in the same coordinate space as the artwork, and
when the real occluding silhouette is already available, prefer element order
over recreating that silhouette as an animated mask. Avoid masking paths that
can remain registered by sharing one transform.

A restricted or interrupted web build can finish Rust compilation without
publishing the wasm-pack output. Serving the previous `.pax/build/debug/web`
directory separately then makes the browser appear to ignore source changes.
This was especially confusing across worktrees because wasm-pack keeps its
version-matched `wasm-bindgen` helpers in a machine-level cache rather than in
the worktree; this branch locked `wasm-bindgen` 0.2.126 while another checkout
still locked 0.2.115. Recommendations: use the canonical `pax-cli run` workflow
with access to the Cargo/wasm-pack caches, require its explicit “Build
completed” message before trusting the preview, and do not diagnose a stale
served cartridge as a template or timeline failure.

Morphing the logo's `a` out from behind the `p` exposed another web masking
boundary. A leaf `Path` works reliably as a mask source, but expressing the
source as either a `Group` of two paths or one compound path intended to union
the `p` silhouette with a reveal half-plane clipped the moving artwork
completely. The animation now uses one simple half-plane path while the `a` is
moving, then hands off to exact unmasked source geometry after its final
endpoint. Recommendations: define and regression-test mask-source composition
for grouped and multi-contour paths; until union semantics are explicit, keep
animated mask sources single-contour and use a stable endpoint handoff when
the final artwork needs geometry outside that contour.

Crossfading two identical filled paths during a geometry handoff caused a
deterministic one-frame opacity dip: at the midpoint, two 50%-opaque layers
compose to 75% coverage under source-over blending, not 100%. In the logo this
made the black `p` counter briefly gray. The duplicate final path was an older
workaround for implicit `Path` clipping, so removing the handoff and retaining
one continuously opaque path was both simpler and exact. Recommendations:
avoid complementary opacity crossfades when identical silhouettes must preserve
coverage; use one continuous path, or an instantaneous handoff when duplication
is genuinely required.

Exposing a reusable animation playhead raised an ownership ambiguity around
autoplay. A direct `bind:` replaces the child field with the consumer's exact
`Property` before the child's mount handler runs, but the public property API
does not retain binding provenance. Starting an imperative ease from the child
on mount would therefore mutate a bound consumer property just as readily as an
unbound default. The logo keeps its normalized `progress` input inert and lets
the embedding component own autoplay, replay, hover, and touch behavior.
Recommendations: treat a bindable playhead as consumer-owned unless the
component has an explicit playback-mode input; if default behavior needs to
differ only when a property is unbound, add first-class binding provenance
rather than inferring it from graph shape or lifecycle timing.

### Slider scrubbing needs both an input event and transition cancellation

**Pain point:** A double-bound `Slider` could write a `Property`, but it did not
emit a typed userland event. If that property was also being driven by
`ease_to`, the active transition would overwrite the user's scrub on the next
tick because a direct `set` did not clear the transition queue.

**Solution:** `Slider` now emits continuous `@slider_change` events carrying the
new value, and `Property::cancel_transitions()` freezes the current eased value
while clearing the active and queued segments. A scrub handler can cancel first
and then re-set the event value, giving the user immediate ownership without
changing the general semantics of `Property::set`.
Removing the retired `pax-designer` crate exposed how a dormant product mode
can survive far beyond its implementation: the release list, workspace
exclusions, feature forwarding, compiler context, generated cartridge
template, proc-macro mount logic, chassis constructors, runtime constructors,
test fixtures, and starter example all retained designer-specific branches.
Several branches were already unreachable or failed deliberately, but still
expanded the release and feature-boundary reasoning surface.

Solved by removing the obsolete crate and its second-manifest build mode
end-to-end while preserving generic designtime inspection, mutation, and hot
reload. Recommendations: when retiring a product mode, trace its semantic flag
through manifests, environment variables, code generation, chassis/runtime
initialization, release automation, examples, and tests; a removed top-level
crate is not a complete removal if its build mode remains encoded downstream.

## 2026-07-25

Applying `#[allow(...)]` to the `include!` invocation for
`.pax/cartridge.partial.rs` did not scope those lints over the items parsed from
the included file. A representative debug build consequently emitted hundreds
of generated naming, dead-code, and unused-binding warnings even though the
macro call appeared to carry narrow allowances.

Solved by making the generated cartridge a private module with its lint envelope
on the module itself, then exposing only the two initialization functions needed
by the surrounding mount code. Mechanically correct diagnostics remain fixed in
the templates, including explicit `Ref<'_, T>` lifetimes and unnecessary mutable
bindings. Recommendations: place generated-code lint policy inside the generated
AST node that owns the code, compile both debug/JSON and release/Rust-manifest
variants in regression tests, and always verify that a warning in hand-authored
user Rust still escapes the generated boundary.

## 2026-07-27

Rectangle corner radii required a verbose helper expression even for the common
uniform case because `#[pax]` always generated object-only coercion for structs.
That also prevented a Pax type from supplying domain-specific literal semantics
without conflicting with the generated `CoercionRules` implementation.

Solved by allowing `#[custom(CoercionRules)]` to suppress that generated
implementation and by giving `CornerRadii` scalar and CSS-arity list
coercions. One radius applies to every corner; two, three, and four values expand
clockwise using CSS border-radius rules. The existing object representation and
helper constructor remain available for compatibility.

While implementing the coercion, placing a private conversion utility inside a
`#[helpers]` impl caused it to be registered as a PAXEL helper and required its
`Result` return type to implement `ToPaxValue`. The macro now registers only
plain `pub` associated functions, leaving private and restricted-visibility
methods as ordinary Rust implementation details. Recommendations: use
visibility as the explicit PAXEL helper boundary, and use custom coercion for
small domain types whose natural authoring syntax is a scalar or list.

Migrating `Font::Web(...)` to contextual literals exposed a second,
compiler-side dependency on the constructor's serialized enum shape: native
font vendoring scanned the manifest for that shape to discover stylesheet URLs.
The runtime coercion was correct, but shorthand-authored fonts would not have
been vendored. The collector now recognizes named `family` and `url` fields
only when they occur in a `font` property context, avoiding false positives
from arbitrary objects. A positional list was considered first, but optional
heterogeneous fields made its indexes opaque; the canonical object instead
names `family`, `url`, `style`, and `weight`. Recommendations: when a literal
representation changes, audit compiler passes that inspect serialized values
in addition to runtime coercion and release baking; prefer type/property
context over guessing a value's meaning from its shape globally.

Using `pax-cli format` only as a parser check during a broad syntax migration
rewrote unrelated whitespace and layout throughout each source file, obscuring
the semantic changes. Running the formatter against temporary copies preserved
the parse validation without expanding the review surface. Recommendations:
expect formatting to be a whole-file operation; when validating a deliberately
minimal mechanical migration, parse temporary copies and leave repository files
untouched unless a formatting pass is itself part of the task.

Repeated `class=...` attributes were the sole intentional exception to Pax's
otherwise single-value inline properties, which made multi-class markup look
like duplicate-key behavior. Classes are now string data: `class="foo"`,
`class=["foo", "bar"]`, and expression bindings that evaluate to `String` or
`Vec<String>`. The binding is reserved selector metadata rather than an
ordinary property, and runtime changes invalidate settings for only that node.
Recommendations: keep one binding as the source of truth across manifests and
baked cartridges; let the runtime own selector matching instead of maintaining
a partial compile-time model of dynamic membership.

Exercising dynamic class changes with `<ImportSettings>` also exposed that
imported selector values and conditions were evaluated against each receiving
node's property stack. Provider-local expressions such as `self.is_dark`
therefore failed outside the theme component, sometimes leaving different
chassis with different-looking fallbacks. Imported settings entries now retain
their provider's lexical stack while `$base` still refers to the receiving
property's previous layer. Recommendations: treat selector matching and value
evaluation as separate concerns, and preserve the authoring scope whenever a
declaration is transported elsewhere for application.

The formatter previously accepted only one path, rewrote it unconditionally,
and omitted the final newline. This made workspace cleanup and automated drift
checks awkward. `pax fmt` now defaults to the current directory, recursively
handles Pax and inline-Rust templates while skipping generated/dependency trees,
supports `--check`, and emits one final newline. Recommendations: keep check
mode side-effect free and report concrete paths so local tooling and CI use the
same formatter contract.

## 2026-07-30

Two retired ICL pipelines remained tracked after their product integrations had
disappeared: the standalone `pax-generation` provider wrapper with duplicated
system prompts, and `scripts/paxgen`, which assembled another prompt from paths
in the former standalone docs repository. Neither participated in the
workspace, CLI, CI, release process, or current agent tooling, but both enlarged
the stale Pax corpus available to codebase retrieval.

Solved by removing both pipelines and preserving only a short provenance note
for the historical Breakout example. Recommendations: keep Pax knowledge in the
canonical docs, examples, and agent instructions; expose observation and
mutation through model-neutral CLI tools; and retire duplicated embedded
prompts when their caller is no longer a supported product surface.

## 2026-08-15

Overriding `Handwriter.stroke` with a partial object containing only `color` and
`width` replaced the component's rounded stroke defaults with the general
`Stroke` defaults (`Butt` caps and `Miter` joins). At larger stroke widths this
produced sharp cusps in otherwise fluid script glyphs. The renderer already
supports rounded caps and joins across GPU and native paths; the example needed
to specify `cap: StrokeCap::Round` and `join: StrokeJoin::Round` in its inline
stroke. Recommendation: remember that an inline object replaces the complete
nested value rather than merging with a component's nested defaults, and make
visually meaningful omitted fields explicit in canonical examples.

## 2026-08-30

Putting the animated Pax logo, whose entrance already uses several `Mask`
components, inside a second expanding quilt `Mask` produced unstable nested
clip behavior as the outer circle changed size, especially after a responsive
resize. Living Quilt now keeps one shared logo card outside the quilt reveal,
so its internal logo masks remain independent while the two quilt scenes change
underneath it. Recommendation: avoid nesting independently animated mask stacks
unless their clip isolation has been verified on every target; a general
layer-level blend or compositing primitive should isolate a subtree offscreen
before combining it with another masked scene.

Growing a repeated list of circles inside a `Mask` correctly produced one
compound coverage path, but the WGPU clip tessellator used Lyon's default
even-odd fill rule. Overlapping circles therefore behaved like XOR: coverage
disappeared under an even number of contours and returned under an odd number.
Clip tessellation now uses non-zero winding so same-direction contours honor
`Mask`'s documented union semantics. A newly inserted sub-pixel circle could
also briefly tessellate to zero indices; the stencil renderer then attempted to
bind cached empty buffers. Empty stencil geometry is now retained as stack state
but skipped at draw submission. Recommendation: test both overlapping and
zero-geometry contours when mask sources add animated repeated children.

The same repeated mask initially expanded its control-flow children without
binding those off-tree descendants to the mask source's layout hierarchy. Every
circle therefore resolved through its default 100-by-100 transform at the
origin: the union existed, but it revealed only a fixed patch while the intended
animation ran invisibly behind it. `Mask` now synchronizes the current sidecar
child sequence into a non-rendered layout tree before resolving coverage, and
repeats that synchronization when keyed children change. Recommendation: an
auxiliary subtree that contributes geometry still needs normal parent/bounds
bindings even when it intentionally skips mount and render traversal; verify
dynamic masks by inspecting their resolved path, not only their child count.

## 2026-09-04

Authoring filled polygon motifs with a list of adjacent `PathElement::Point`
values parsed successfully but produced no visible faces. After the first point,
another point starts a new subpath; an explicit `PathElement::Line` must precede
each destination that should connect to the current contour. Recommendation:
when a custom path has bounds and appears in scene inspection but does not fill,
inspect its command sequence before debugging layout or lighting.

The same pass exposed that a helper call is not accepted as a bare inline
property value: `material=Material::matte()` fails parsing while
`material={Material::matte()}` correctly enters PAXEL expression syntax.
Recommendation: brace helper calls in templates even when the result is a
constant value.

Formatting long Pax elements wrapped attributes onto continuation lines with a
space left at the preceding line ending, causing `git diff --check` to fail.
The affected starter templates were expanded manually after formatting.
Recommendation: keep whitespace validation separate from formatter check mode
until wrapped element output is guaranteed to be trailing-space free.

## 2026-09-06

Geometric `Mask` coverage deliberately ignores paint alpha, so translucent
strokes and gradient stops cannot feather a reveal. `Mask alpha=true` now uses
painted vector alpha on WGPU, with an optional `feather` Gaussian sigma in
logical pixels. Use source-over union for repeated sources and test even as
well as odd overlaps; source-side clips and native control content remain
outside this first alpha-mask implementation.

Inside an object literal, a dynamic stroke width needs its own expression:
`stroke={color: WHITE, width: {(ripple.band_width)px}}`. A bare parenthesized
binding there is parsed as a static literal and fails. For fractional grid
positions, use floating-point arithmetic (`row * 100.0 / 3.0`); integer
division truncated thirds and left an uncovered strip at the viewport edge.

## 2026-09-07

Living Quilt republished a `Property<Vec<TileState>>` on every slide frame, so
motion invalidated the repeated shape data and reconverted unchanged paths to
PAXEL values. Keep repeat membership and immutable geometry on a structural
signal; animate a mounted component's small x/y properties separately. Use
`set_if_neq` so settled panels stop propagating updates, and `Property::read`
when inspecting unchanged paths rather than cloning them with `.get()`.

Do not assume nested `Property` handles survive expression conversion:
`Variable` exposes a cached `PaxValue` snapshot, and derived struct conversion
reads wrapped fields into ordinary values. Living Quilt uses `QuiltPanel`'s
top-level motion properties instead, preserving the existing debug and baked
release binding contract without adding a new nested-reactivity feature.

## 2026-09-08

Living Quilt's expanding ring could leave a frozen color fragment once panel
motion settled; mouse movement cleared it. `Mask` collected source dependencies
only at mount, when the repeated ring list was empty. The off-tree source's
later paint updates and final removal therefore did not invalidate the visible
content. Mask sources now keep recursive subscriptions to child membership,
layout, paint properties, and computed opacity, retaining watches for keyed
children and releasing them on removal. Recommendation: test auxiliary render
trees from an initially empty repeat through insertion, animation, and removal
with no unrelated input or animation driving redraws; continuous redraw is not
a substitute for tracking those dependencies.

## 2026-09-09

Living Quilt's light briefly resisted mouse movement after a click because the
420ms click/touch tween continued overwriting direct `Property::set` updates.
Setting a value does not cancel its transition. The mouse-follow handler now
calls `cancel_transitions()` on both coordinates before setting them, so pointer
input takes over immediately while touch keeps its eased movement. Recommendation:
explicitly cancel animation when direct manipulation takes ownership of the
same property; do not mistake competing writers for a rendering-cache delay.

The mask-source ring optimization first tried a reusable component with derived
properties in `Default`. Its pure Rust test passed, but its Path never appeared
in the browser: off-tree control-flow expansion does not mount an ordinary
component's template. For this case, direct repeated Path leaves with pure
`#[helpers]` functions are sufficient. Immutable ring descriptors change only
at birth/removal, while clock and bounds drive each leaf's geometry independently.
Recommendation: verify off-tree source composition in a rendered app, not just
by testing component properties; full component support there is a separate
lifecycle feature, not something to approximate with hidden mounted copies.

A `Property<(f64, f64)>` component field also failed macro/static analysis during
that experiment. Separate scalar width/height properties work through both
authoring paths. Recommendation: use supported scalar fields for scene bounds
until tuple-valued component properties have an end-to-end binding contract.

## 2026-09-11

Reusing alpha-mask GPU buffers requires upload ordering, not just allocation
capacity checks. `Queue::write_buffer` transfers run before submitted command
buffers, so multiple encoded versions of one mask could all read the last
upload. Mask uploads now use the renderer's staging belt with copies ordered
before each consuming pass. Retained capacity is separate from current draw
ranges; empty masks still clear coverage, and filter groups resolve current
parent textures instead of retaining stale bindings. GPU snapshot tests capture
intermediate versions before submission, including growth, shrinkage, removal,
parent replacement, and viewport changes.

Surface resize discards pending command buffers. Their encoded mask signatures
must also be invalidated, and their staging allocations discarded rather than
recalled as submitted work. Otherwise a retry with identical inputs can falsely
hit a cache entry whose GPU upload never ran. Recommendation: test cancellation
as well as successful submission when adding retained rendering resources.

## 2026-09-12

Living Quilt's animated-to-static logo handoff could disappear for one frame
when a tile panel retired at the same time. The runtime advanced its animation
clock after settling effects and layout. A conditional could consequently
replace its children during render traversal, after the retained draw plan had
captured the old node IDs. Concurrent removal forced a scene clear, exposing
the missing replacement. The final effect drain and occlusion/layout pass now
follow clock advancement and queued custom-event dispatch, so rendering sees
the settled tree. Deterministic non-GPU tests cover the handoff with and without
retained replay and sibling removal. Recommendation: settle structural effects
before snapshotting render work; overlapping masks can expose a frame-ordering
bug without being its cause.

Concurrent Living Quilt rings amplified keyed-repeat work: retained groups wrote
unchanged item/index values, reattached children, and rebuilt layout subscriptions.
Repeats now compare payload representations exactly, retain parent bindings while
their source identities match, and gate structural work on rendered/active/exiting
child sequences. Do not use `PaxValue::PartialEq` to suppress data propagation:
`Numeric` uses approximate language equality, which can hide small real changes
inside paths, colors, units, or nested item data. This optimization deliberately
does not change PAXEL equality or general `Property::set` semantics.

Parent bindings must be compared by source identity, not their current values;
reload and explicit bounds rebinding invalidate the fast path. Structural checks
must evaluate reconciliation first and distinguish active from exiting children:
starting an exit can change slot projection without changing rendered node IDs.
Clock-only exit cleanup must still publish the final removal. Recommendation:
test no-op work counts alongside output, nested repeats, off-tree mask geometry,
equal-valued replacement sources, and repeated churn returning live property
counts to baseline. Keep lifecycle work in the existing settlement lane rather
than moving effectful reconciliation into propagation-cutoff evaluation.

Simple same-type bindings could repeatedly convert nested Rust records into
`PaxValue` object trees and then immediately coerce them back. The shared rich
and release-baked property builders now forward audited plain values through
independent computed properties, while consuming owned expression results
without another defensive clone. This is not aliasing or zero-copy transport:
each dirty typed hop still clones its Rust value, and child writes remain local.
Arithmetic, accessors, unit conversion, sparse literals, and double bindings
retain their existing paths.

Exact Rust type identity alone is insufficient for this optimization. Custom
coercion can normalize values, property-wrapped fields snapshot mutable state,
and the current derive omits non-path fields from value conversion. The opt-in
contract therefore defaults to false, derives only for fully represented plain
records/enums with generated defaults and eligible fields, checks actual erased
property storage, and rejects recursive capability cycles. Recommendation:
test conversion counts and mutation isolation alongside values, custom fallback,
float bits, evaluator replacement, rebinding, and create/drop lifetime counts.
Keep eligibility in the shared runtime path so baked cartridges do not acquire
a separate expression or serialization contract.

## 2026-09-13

Profiling every property read substantially slowed the ten-ring workload and
changed how arrivals batched into frames. Narrow, separate release traces for
value transport and reactive settlement were more informative than one heavily
instrumented trace. Listener names can also misattribute lazy work: a mask or
layout-hull callback can trigger repeat reconciliation and node construction.
Recommendation: report inclusive and exclusive scope time, operation counts,
actual input delivery times, and an uninstrumented baseline. Do not equate a
whole listener's time with its final invalidation operation, or use intrusive
probe frame times as a production performance comparison.

Property replacement's dependency reconnect is not redundant merely because
the dependency IDs match: reconnecting moves the destination to the end of
upstream outbound lists, whose traversal order feeds effect scheduling, and
connecting must enqueue dirty upstream cutoffs. The first binding-loop cleanup
therefore preserves those operations. It reuses the destination dependency
vector's capacity and borrows the inbound slice during connection instead of
cloning it. That borrow is safe only while connection edits graph metadata and
queues work without evaluating user code. Capacity belongs to the property and
is released when it drops; there is no shared binding cache. Regression checks
cover reordered/duplicate/empty dependencies, evaluator replacement, shared
targets, connection order, and property cleanup. Repeated ten-ring release
captures did not establish a substantial frame-time gain from this narrow
allocation reduction; avoid presenting it as a substitute for reducing the
amount of node initialization work.

Template-node initialization used to resolve settings independently for common
and component properties both before and after allocating the expanded node.
It also built two property scopes, including their conversion-adapter
properties. Structured template initializers now share an immutable input plan:
allocate slots, establish selector identity, resolve once with node context,
bind common and component properties, publish the final scope, then activate.
The plan contains no resolved selector matches, provider bindings, or computed
values; rebinds resolve afresh, and sibling repeat nodes never share live slots.
Runtime defaults and per-field custom-default applicators are not pooled or
coalesced. The plan is constructed from existing manifest data in the shared
traverser, so rich, Rust-emitted release, and binary-decoded manifests do not
need a new wire format or descriptor schema.

One prebinding phase cannot yet be removed indiscriminately. A component-local
timeline can capture a sibling slot before a later descriptor installs a
double-binding alias there, leaving the timeline attached to the old slot.
A regression test reproduced that failure. Such local timeline/transition
nodes retain prebinding, as do custom property or scope factories (including
mixed structured/factory initializers). Removing that compatibility path needs
an explicit alias-installation phase before local-scope capture, not skipped
invalidation or a special case in the example. Tests cover forward aliases,
selector removal/reset, imported-provider scope and `$base`, lifecycle order,
reload, shared-plan isolation, and property cleanup. Repeated ten-ring release
comparisons improved, but input batching and short overlap windows still make
an exact speedup estimate noisy.

## 2026-09-14

The physical-device identifier shown by `xcrun devicectl list devices` is a
CoreDevice UUID, not the hardware UDID currently matched by Pax's
`--ios-device` selector. Passing that UUID failed even though the attached iPad
was paired and available. Use `device:<name>` or the hardware `udid` reported by
`xcrun devicectl device info details --device <CoreDevice UUID>` instead.
Recommendation: distinguish these identifiers in device-selection diagnostics,
or accept both through an explicit mapping when extending the selector.

The same device check found an obsolete release guard after successfully
compiling the optimized Rust framework. Removing it alone was insufficient:
the iOS template has Debug and Release configurations under the same
`Pax iOS (Development)` scheme, and its old Release signing default was
`Don't Code Sign`. Local `run --release` now uses that existing scheme,
Apple Development signing for devices, and no signing for simulators. It
forces designtime and hot reload off; App Store archive/export/upload remain
separate work. Install/launch also needs the effective metadata bundle ID,
not just the project-derived default that metadata may override in Xcode.

The bundled mobile AppIcon catalog listed a 1024x1024 slot without a filename
or image. Apps without icon metadata consequently showed a generic or blank
icon in both build modes. The template now carries the current opaque Pax
icon (reused from the increment example); custom metadata still replaces it.
Keep the bitmap inside the compiler crate's packaged interface, not behind a
build-time dependency on monorepo example files.

Living Quilt's ambient tile cadence is 500 ms, but each slide lasts 580 ms.
Requiring every panel to settle before each ambient update would stall that
cadence. Idle is now latched after the interactive
scene and logo settle; ambient slides preserve that state, and an accepted ring
clears it. Idle panel batches and rings also share one ID sequence so their
keyed panel repeats cannot collide. Schedule the next deadline from the current
time, rather than replaying missed timer events after suspension.

Logo replay is a separate `@click` handler on the logo card. Its click bubbles
to the canvas for one ring attempt, so the card can replay without throttling
even when the ring cap is full. Automatic rings and clicks outside the card
never reset the logo. Keep replay out of the shared ring-emission helper.

## 2026-09-15

Living Quilt's unoptimized debug build spent far more CPU time in a five-ring
burst than release. Optimizing every crate at level 1 recovered runtime speed,
but made small Rust application edits substantially slower to rebuild.
Keeping runtime dependencies at level 1 and the application at level 0 retained
near-release web performance while preserving quick application rebuilds. The
trade-off is a longer first dependency build, not disabled debug assertions,
overflow checks, debug information, or hot reload.

Express this as `[profile.dev] opt-level = 1` plus a named application package
override at level 0. A wildcard package override also overrides Cargo's
build-script/proc-macro defaults and is not the same policy. When creating a
project from a bundled example, rename the example's package profile overrides
alongside `package.name`; otherwise Cargo silently leaves the new app at the
profile-wide optimization level. Canonical manifests, bundle drift checks,
and renamed-project creation tests now cover this boundary. Keep
`CARGO_PROFILE_DEV_OPT_LEVEL=0` available for fully unoptimized debugging.

## 2026-09-16

A detached Windows telemetry worker kept captured CLI output pipes open even
with its own stdout/stderr set to `Stdio::null()`. Stable Rust's `Command`
also inherits other inheritable handles, including the original standard
streams. Clear only those streams' inheritance flags once at CLI startup,
before threads start; do not close or replace them. Explicit `Stdio::inherit()`
still duplicates the handles for normal compiler children. Validate both
foreground exit and pipe EOF while the worker's HTTP response is withheld.

The loopback telemetry fixture also intermittently dropped requests on macOS
and Windows because accepted sockets retained the listener's nonblocking mode;
Ubuntu did not reproduce it. Explicitly set accepted `TcpStream`s to blocking
before using blocking readers and read timeouts. Keep fixture timeouts longer
than the client's request budget so the fixture does not manufacture failures.

Web builds can load a project's `.env` into the CLI process. Capture telemetry
endpoint, consent-directory and suppression settings before command work,
including originally absent values, so a later worker cannot accidentally use
project-supplied configuration.

## 2026-07-22

Building the responsive Pax website exposed a positioning assumption that is
easy to carry over from CSS: percentage `x`/`y` positions are evaluated over
the element's remaining travel, not directly against the full parent extent.
For example, a wide child at `x=64%` will not begin at 64% of the parent.

Solved by using anchored intent for relative placement (`x=50% anchor_x=50%`
to center and `x=100% anchor_x=100%` to right-align), and by calculating pixel
positions from `bounds_self` when a responsive two-column split needs exact
full-parent coordinates. Recommendations: call out percentage travel semantics
next to the positioning examples in the layout docs and add a small diagram
showing left, center, and right anchored placement.

The same site uses `ExampleHost` as a public source-inspection affordance. On a
narrow viewport the drawer correctly expanded to the full host width, but a
zero-width live preview could still composite native text above the drawer.
Shrinking or covering the preview was therefore insufficient on mobile.

Solved by clamping the drawer to the host width and not instantiating the
preview subtree while the compact, full-width drawer is open. Recommendations:
for responsive overlays that cover arbitrary projected content, verify native
elements as well as canvas primitives; visual width and opacity do not by
themselves establish a native-compositing boundary.

## 2026-07-23

Adding in-situ route metadata exposed a parser boundary: typed literal blocks
such as `RouteMetadata { ... }` were eagerly converted into the generic
`PaxValue::Object` form, which discarded the author's declared type before a
specialized consumer could validate it. The values survived, but the compiler
could no longer distinguish a deliberate metadata block from an arbitrary
object literal.

Solved by preserving explicitly typed literal objects as structured
`ValueDefinition::Block` values during template parsing, then validating the
required literal fields at the `Route` boundary. Recommendations: retain source
type intent until the consumer that owns the schema has run; generic value
coercion should not erase information needed for compile-time validation or
static analysis.

## 2026-08-03

Building a data-driven website card rail exposed two authoring details that are
easy to infer incorrectly from CSS and Rust conventions. Pax settings do not
accept comma-separated class selectors, so a rule such as `.axis, .track` fails
template parsing and must currently be written as two rules. Pax data records
that need generated field defaults use `#[custom(Defaults)]` (plural), while
`#[custom(Default)]` expects the record itself to provide `Default` and fails
during generated Rust compilation.

Solved by expanding the shared selectors and using the established
`#[custom(Defaults)]` annotation for repeated record data. Recommendations:
document both constraints near settings syntax and repeated-data examples, and
consider making the singular/plural default error explain the intended choices.
