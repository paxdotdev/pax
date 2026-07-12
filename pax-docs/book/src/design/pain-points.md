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
The `svg-path-drawing` example now includes multiple closed contours to keep
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
