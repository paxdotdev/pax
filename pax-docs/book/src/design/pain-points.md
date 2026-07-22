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

Watcher and designer edits initially still had a stale-buffer race: parsing a
file happens outside the coordinator lock, so a slow watcher callback could
commit an older disk buffer over a newer designer write. A single global
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
designer has already changed the file.

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
active. Recommendations: inspect the generated designtime manifest when an
animation contradicts its source, and treat timeline-definition edits as
requiring a clean rebuild until timeline hot reload is fixed.

Frame-sequence diagnostics also had two capture traps. `pax-cli dev look`
returned black web-canvas frames in this example, while browser screenshots
used changed-region optimization that could make a valid animation look like
detached fragments. A temporary fixed-step playhead plus a full-screen,
playhead-driven background change produced reliable full frames and isolated
the rendering failure. Recommendations: provide an official fixed-playhead
capture mode for declarative timelines and an option to force full canvas
frames in animation tooling.

Refining the logo post from a translated rigid shape into an unfurling fabric
strip required animating `PathElement` geometry procedurally. Rebuilding the
same cubic-path topology from a shared playhead rendered reliably; changing
only control points and endpoints also made exact contact-sheet frames easy to
compare. The lower-edge overshoot was initially clipped at the `Path` bounds,
so the component now reserves four percent of its internal vertical coordinate
space and compensates in its presentation height. Recommendations: keep
procedural animation topology stable, and give authored overshoot explicit
geometry headroom instead of relying on drawing beyond primitive bounds.

Moving that post animation from an imperative Rust playhead to `@timeline`
worked without interpolating `Vec<PathElement>` directly. The timeline owns a
small set of scalar motion controls (extension, wave, swing, whip, and spool
radius), while one reactive Rust computation maps their instantaneous values
into stable-topology paths. This keeps timing, easing, and choreography visible
in Pax while retaining path assembly and screen-space geometry math in a pure
adapter. Recommendations: expose artist-facing scalar controls to timelines
before adding collection interpolation or custom expression helpers for
procedural geometry.

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
