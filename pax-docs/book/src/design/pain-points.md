# Pain points — raw findings inbox

This is a temporary intake for findings encountered during work, not a reference
or a prerequisite for agents to read before starting a task.

Add concise, dated reports: what you tried, expected versus observed behavior,
reproduction steps or evidence, relevant target/revision, and any workaround or
related Linear issue. Link repeated reports instead of copying long journals.

Periodically triage the inbox: discard resolved or obsolete findings, preserve
ongoing issues and report details in Pax Core Linear tickets, and remove each
processed entry. Keep only exceptional proactive guidance in `AGENTS.md`.
Design exploration and specifications belong directly in Linear, not here.

Last triaged: 2026-10-05 — [PAX-972](https://linear.app/paxdev/issue/PAX-972/pay-down-the-pain-pointsmd-file).

## Untriaged findings

- 2026-10-29, website acrostic outlines: positioning vector letters at the
  font's cap height clipped rounded O/C overshoot in the top Frame row. Expected
  complete contours. The website now fits each decorative letter's full control
  bounds into an inset ink box, reserving space for its heavier outline; the
  hero retains proportional font shaping. Regression coverage checks normalized
  bounds and equal gutters in `pax-website/src/acrostic.rs`.

- 2026-10-28, website first-scroll profiling: mounting a frozen
  `AnimatedPaxLogo progress=1` in the revealed navigation adds three Mask clip
  boundaries and falls back to a whole-page occlusion rebuild. At 1800×1043 in
  release Chrome, coverage alone took 47–49 ms; replacing that nav instance with
  `StaticPaxLogo` retained over 5,700 unchanged nodes and reduced coverage to
  2.8–3.4 ms. Reproduce by loading the homepage and scrolling until the nav
  appears (or hiding/revealing it again). The website workaround preserves the
  hero animation; incremental reconciliation of newly mounted clip subtrees
  remains a possible engine improvement, not implemented here.

- 2026-10-25, PAX-1047 modal capture: `pax-cli dev look` of the oscilloscope's
  open message editor omits the dark native `EventBlocker` backdrop and puts
  the native OK label at the button's left edge. Expected the same image as
  the browser, where the backdrop is visible and OK is centered. Compare the
  browser observation in the PAX-1047 chat with
  `/Users/zack/.codex/visualizations/2026/10/25/01a1729c-65d2-76e3-a526-2ce0d0d226df/message-editor-mobile/0000.png`.
  Used the browser directly to verify dialog appearance; the composite capture
  is not reliable evidence of native opacity/alignment here.

- 2026-10-25, PAX-1047 shared build cache: after building the standalone
  oscilloscope and shader-enabled website into the same `CARGO_TARGET_DIR`,
  the website failed in `pax-macro/src/source.rs:232` with “conflicting
  configuration for a dependency source”: `feature = "shaders"` was true
  through one dependency and false through `led_oscilloscope`'s source helper.
  Expected Cargo's feature-specific artifacts to remain coherent. Touching
  `examples/led-oscilloscope/src/lib.rs` and rebuilding the website serially
  succeeded without source changes. Exact cache-invalidation cause is not
  established; the compiler diagnostic is in the PAX-1047 chat, and the successful
  rebuild is `/private/tmp/pax1047-website-resume.log`.

- 2026-10-25, PAX-1047 web preview: after replacing a borrowed CLI with this
  checkout's CLI and rebuilding with `--libdev`, an existing browser origin
  still showed native knob labels but a blank canvas. Fresh Chrome and a fresh
  localhost origin rendered the identical standalone build. The first website
  load also omitted scroller artwork (including the existing Path Study); a
  rebuild and fresh navigation restored it. Do not diagnose app geometry from
  a mixed/cached host and cartridge. The standalone stale-origin comparison is
  confirmed; the website symptom's cause is not established. Evidence:
  `/private/tmp/pax1047-web.log`, `/private/tmp/pax1047-website-rebuild.log`;
  clean desktop and mobile captures are recorded with PAX-1047.

- 2026-10-20, PAX-839 web checkpoint: after a fresh debug load at 1440×900,
  native content appeared but canvas artwork stayed blank through a gallery
  tour and return to the hero. Resizing to 720×900 restored the artwork;
  another load resized during startup rendered correctly. The same symptom
  occurred in pre-fix tracing builds. Release verification also found missing
  tablet artwork after a 1280×720→1440×900 resize, with native text still
  visible and no console errors; reloading at 1440×900 restored it. Evidence:
  `/tmp/pax839-release-missing-artwork.jpg`. Recheck retained-surface publication
  with PAX-1014 before calling the visual merge gate complete; cause not established.

- 2026-10-20, PAX-839 web checkpoint: the zero-scale gallery invalidation storm
  is fixed, but 1440→720px responsive resizing and a 720→1920px startup resize
  still reached the 100,000-evaluation reactive drain budget. The scene settled;
  no recurring SVG errors were observed. A resized run also emitted two
  `pax-tile-window-escape` warnings. Keep these distinct from the resolved
  nonfinite autosize cascade when checking PAX-1014 integration.

- 2026-10-15, PAX-839 web host validation: building `pax-website` without
  `--libdev` rebuilt the cartridge but reused an existing browser interface
  bundle after `interfaces/web/src/index.ts` changed. The older host treated
  the new partial startup checkpoint as full readiness. Monorepo website
  development requires `pax-cli build --path pax-website --target web --libdev`
  (plus `--release` for release); automatic libdev detection covers examples
  and tests, not this website. Confirm the rebuilt host and cartridge together.

- 2026-10-13, PAX-839 retained-gallery integration check: one FeatureStudy
  exit timeout occurred under build load at 1440×900. Card identities stayed
  stable through forward/reverse traversal and 720/1920px resizes. Evidence:
  `/tmp/pax839-gallery-20261013/integrated-release-1440.json`. The invalid SVG
  paths and repeated animation-driven drain warnings from that run were traced
  to collapsed transforms and fixed on 2026-10-20; the exit timeout remains
  an independent follow-up.

- 2026-10-13, initialization rebase: directory renames did not migrate Shader
  Studio, which main added after the example-layout branch diverged, or rewrite
  the website’s relative Paxflix asset symlinks. Compare incoming added files
  against their final paths and check symlink targets, not only textual conflict
  markers. Move the new fixture into the Cargo/example discovery layout and
  repair asset links before regenerating bundled sources and building the site.

- 2026-10-12, pre-merge validation: a website build did not compile a new
  `pax-std` autosize test, which still constructed runtime `CommonProperties`
  and `ConditionalProperties` with shared `Property` values after the property
  migration. These internal fixture fields require `LocalProperty`; authored
  component fields still use `Property`. Run the affected library's `cargo test
  --lib` after rebasing test fixtures, as an application build cannot catch this.
- 2026-10-12, website hero: an interactive `AnimatedPaxLogo` used a one-way
  `progress={self.logo_progress}` expression alongside a per-frame progress writer.
  Keep its replay controller and parent on the same `progress=bind:logo_progress`
  playhead; start the initial reveal with a transition, then let the component's
  default `click_to_replay=true` handler own subsequent transitions. Replay also
  exposed a shared/local migration bug in the component: `progress.set(0.0)`
  publishes a reset, but an immediate `progress.local().ease_to(...)` still starts
  from the old UI value. Reset and ease through the same local view. A regression
  test reproduced progress staying at 1.0; it now covers replay from rest and midway.

- 2026-10-12, website hero: `creative` used `opacity=0` on its native backing
  Text, hiding the selection highlight along with the glyphs. Use transparent
  text fill with normal node opacity, `selectable=true`, and `clip=true` instead;
  clipping also selects the interactive iOS/iPadOS text path. The Manrope
  selection remains approximate under EMS League handwriting.

- 2026-10-12, website / PAX-1030: an application-local shader primitive declared
  with `#[primitive("pax_website::hero_metal::HeroMetalInstance")]` passed static
  analysis but failed the generated cartridge with E0433 (the application cannot
  import itself by dependency name). Use `crate::hero_metal::HeroMetalInstance`
  for a primitive defined in the application; library primitives use their crate
  import path. Verify custom primitives with a cartridge build, not only Rust tests.

## 2026-10-06 — macOS display-link startup

A macOS example can open a blank window if CoreVideo reports
`CVDisplayLinkCreateWithCGDisplays error -6661` with an active display count of
zero. In the Scroll Matrix smoke test, relaunching after the display became
available restored rendering without source changes. Inspect the app's system
log before treating an idle blank window as a renderer regression. The current
display-link setup does not retry that startup failure automatically.

- 2026-10-11, shader-studio / PAX-1035: PAXEL does not accept Rust method calls
  such as `self.phase.sin()`. Expose a `#[helpers]` associated function and call
  `Landscape::sun_x(self.phase)`; grouped units work as `{(Landscape::sun_x(self.phase))%}`.
  Numeric literals need a fractional digit (`24.0`, not `24.`). Frame's
  `corner_radius` is `f64`, so use `18`, not `18px` (the latter warns and keeps
  the default).
- 2026-10-11, shader-studio web debug build: a fresh resolution to Pest 2.9.3
  introduced the psm Wasm stack helper. macOS `ar`/`ranlib` warned that `wasm32.o`
  was not Mach-O; the emitted JS imported unresolved `env::rust_psm_stack_pointer`
  and the browser stayed blank. Install Rust's official `llvm-tools-preview`
  component and set `AR_wasm32_unknown_unknown` to that toolchain's `llvm-ar`
  for the web build. This links the real helper and removes the `env` import.
- 2026-10-11, shader-studio / PAX-1035: `pax-cli dev look` places the styled
  mode-button labels at the upper left, although the live browser centers them
  with `TextAlignHorizontal::Center` and `TextAlignVertical::Center`. Reproduce
  with the example's `.mode_button` class and compare a dev-look PNG to a browser
  screenshot. Use the browser to judge native-control alignment; the capture
  remains useful for inspecting GPU brush pixels. After hot reload and scrolling,
  dev-look captures also showed native buttons/sliders duplicated at old positions
  while the live browser remained correct.

## 2026-10-12 — Moving shader paint can resize a fixed container's capture

The website's Signac letters allocated two new shader outputs on almost every
painted frame. Their radial-gradient Ellipses moved beyond the fixed container,
changing its capture envelope. The exit also rotated and shrank the entire
filtered word. Capture dimensions include child bounds plus sampling support;
a fixed layout box alone does not make them stable.

Animate radial centers inside fixed Rectangles, or keep moving discs within the
field's bounds. Freeze source motion and phase during a stationary ancestor fade
to reuse the painted result. The renderer now retains same-sized capture textures
when their origin changes and updates shader coordinate uniforms without replacing
pass textures. Dimension changes still allocate, and transform changes still
invalidate rendered pixels. Use resource-churn traces to distinguish allocation
from shader work; they are not frame-time measurements.

## 2026-10-01 — Warm admission and canvas turnover are separate budgets

The two scaled Paxflix embeds exposed intermittent black rows even after fixing
scroll extents and mounting cards ahead of view. At native 2x density, diagnostics
recorded checkouts against all 64 allocated canvases, plus delayed first draws.
Warm admission compared screen-space bounds against unscaled viewport sizes, and
canvas reconciliation requested replacements before retiring other layers' tiles.

Use presentation-space sizes consistently, distinguish actual ancestor-clipped
visibility from an offscreen warm runway, and retire obsolete assignments across
all layers before allocating. Refresh retired layers synchronously so their GPU
owners can release; do not remove the lease guard for asynchronous initializers.
Visible hosts get allocation and queued-initialization priority, with FIFO ties
and current visibility sampled when a slot opens. Preserve survivor canvases and
screen-density backing stores. Verify first draw during scrolling, not just a
settled screenshot; application tree mounting and image work can still stall a
frame independently of canvas admission.

## 2026-09-29 — Small live studies inside a virtualized gallery

A nested Rust component imported as `use paint_demo::FeaturePaintDemo` passed
Rust tests but failed Pax static analysis with an unresolved type. Use the
crate-qualified path (`crate::feature_gallery::paint_demo::FeaturePaintDemo`)
and expose the module so generated cartridge code can resolve it too. Test
the actual web build as well as the Rust library.

Mask sources do not receive viewport-proximity events. Keep the animation clock
on the visible demo wrapper, use its clipped global viewport sample to pause
offscreen work, and bind progress into the source. This covers both the page's
vertical clipping and the rail's horizontal clipping without polling geometry
per frame. Keep meaningful text alternatives outside the detached mask source.

For moving CMY paint through handwriting, repeat the paint period and translate
the shared parent instead of regenerating the letter paths. Keep the paint clock
separate from a signboard clock that snaps to a legible word on pause; otherwise
pausing also jumps the colors. Palette indexing in PAXEL uses `%%` for modulo;
the single `%` token denotes a percentage unit.

For the subsequent one-shot pinstripe treatment, scale across the stripes before
rotating them so their 45-degree axis stays fixed. Compute the rotated coverage
when panning them out; moving just their center past the word can still
leave a colored tail. Keep the white underpaint and alpha source unlit, and scope
the stripe light with LightFrame so the accent does not relight the rest of the
page. The visible wrapper owns the clock; pause can finish a one-shot to its clean
resting state instead of freezing a bright glint.

When those bands must remain contiguous, use one rectangle with hard CMY gradient
stops instead of separate spaced rectangles: scaling also scales any authored
gaps. Pipeline the glint with writing on the same clock. For a diagonal ribbon,
compute its final travel against the vertical slice occupied by the mask, not
the full long rectangle; otherwise its visible portion exits before its large
off-mask ends, making the effect seem to finish early.

When replacing a lighting study with flat moving paint, explicitly use unlit
material on both the content and mask source; deleting the local light alone
would still allow other scene lights to influence a default-lit shape. The
cycling gradient placeholder reuses the mask-strokes fixture's two repeated
CMY periods and translates one period in four seconds. Keep this paint clock
independent of writing completion, so pause finishes the lettering without
jumping its colors. Preserve the parked pinstripe as a clock-free component
instead of leaving its per-frame work mounted invisibly. Genuine mesh-gradient
paint is implemented separately in PAX-1011; overlapping radial gradients and
this repeating linear gradient are not substitutes for it.

When rebasing component-store tests onto the Paint/Fill split, migrate recorder
trait signatures to `Paint`, implement the image-opacity entry point, and use
the shared designtime-aware runtime/engine fixture constructors. A default-only
test run would miss the latter integration issue.

## 2026-09-29 — Embedding a complete example without a second runtime

Paxflix exposed a related composition trap: a root component can use local
bounds for its main layout while detail dialogs and theme font sizes still use
`$viewport`. Two device previews then disagree with their visible containers.
Bind the app's logical width/height from `ctx.bounds_self`, pass those values to
its detail panel, and pass explicit responsive flags into its settings provider.
Keep mock safe-area ownership and global keyboard opt-out explicit, as below.
The current static analyzer collects the root project's `assets/`; a Cargo
dependency alone does not bring its media. A repo-relative asset symlink provides
one source inventory for the website and standalone example; the web builder
copies it into normal deployment files. Preserve symlinks when checking out the
monorepo, and audit path collisions before adding another image-heavy example.

Do not assume a Rust method named `on_mount` runs automatically. Paxflix's
shelf/card subscription methods existed, but their templates omitted `@mount`
bindings, leaving the scheduled titles invisible. Wire lifecycle setup and
cleanup explicitly, and include a live visual check beyond scheduler unit tests.
For a large embedded catalog, keep shelf footprints fixed and conditionally
mount each shelf's card subtree with a preparation buffer in the catalog's
local scroll coordinates; retain it after the first visit to preserve state.
Global proximity is not an offscreen prefetch API through hard Frames: the
device clip truncates its expanded region. Use the local scroll offset and
viewport height to prepare rows before they enter that clip. This reduces
initial scene expansion without claiming full virtualization or bounded
retained memory.

Browser-native overlay hosts cancel their Scroller's world transform. Under
downscaling, their inverse-scaled boxes can inflate CSS scrollable overflow
even when the Pax content extent is correct. Bound that overflow at the
untransformed inner pane with `overflow: clip`, preserving the explicit pane
size and the outer native scrolling element. Do not compensate in application
scroll dimensions or add paint containment to nested canvas hosts.

Expose the calculator as a normal component and keep a thin standalone root;
a Cargo path dependency lets the website reuse its actual source, model, and
per-instance store. A viewport-proximity host can defer the first mount, then
retain session state while gating the example's update handler offscreen. This
is not general subtree suspension: mounted nodes and retained GPU resources
still exist after the first visit. Measure heavier examples independently.

An embedded calculator needs an explicit keyboard policy: @key_down delivery
is global, so an offscreen mounted example must not consume the page's keys.
Disable its keyboard handler in this host while keeping on-screen controls live.
Let the host supply logical safe-area insets for a mock phone rather than
letting zero-valued web DynamicIslandSpacers overwrite them. Keep the background
full-bleed and inset only the controls. Put the island before the clipped slot
in Pax source so it renders above the app. Suppress layer-wide AmbientLight in
embedded mode; a LightFrame alone does not localize that lighting resource.

Frame uses `corner_radius`, not `border_radius`; the latter did not round the
website's old device clip. Keep the hardware outline and actual clipping shape
aligned. For mouse dragging inside a transformed example, convert both the
initial and current window points through the same `ctx.local_point` and then
multiply their delta by local bounds; raw window deltas under-pan a scaled app.

## 2026-10-01 — Bounded artboards in a projected feature rail

Fixed-pixel artwork authored for a 288px-wide area overflowed compact cards'
254px media wells. Normalize projected artwork to one logical artboard and
uniformly scale it into the available width, inside a Frame. Reserve a separate
caption strip; clipping alone would merely crop the intended composition.
Use percentage units for scale: a numeric 1 is not equivalent to 100%.

A caption positioned at `100% - 28px` inherited a percentage anchor and moved
up into the art. An explicit pixel Y and `anchor_y=0%` made the contract clear.
Check foreground-first Pax source ordering before blaming the compositor:
several old opaque poster backgrounds were above their own labels.

Gate each study clock with viewport-proximity notifications from the visible
artboard, resetting its last timestamp on visibility changes. Retain path
geometry and vary draw ranges/transforms; keep native scrolling in the parent
rail so nested illustrative scenes do not capture page gestures.

Validate named easing curves in the running app, not just with the template
parser: unsupported names can compile but log a warning and fall back to
Linear. The card transitions use the documented OutQuad/InQuad pair.

## 2026-09-30 — Shared lighting across embedded and standalone layouts

The calculator's desktop body tracked the pointer, but its full-bleed branch
removed that light and substituted a fixed-position light around only the
background. The mouse handler still updated properties that no mounted light
used; sibling LightFrames also kept the background light off the keypad.

Use one calculator-local LightFrame around both the scrolling body and the
full-bleed background, with a single shared point light. Bind pointer movement
on that frame and convert window coordinates through `ctx.local_point` there,
not in the inner scrolling body. Initialize the light in the same viewport
space, accounting for safe insets and the centered desktop case. Keep embedded
AmbientLight omitted because ambient contribution remains layer-wide, and
retain the existing offscreen pause gate on the direct light.

## 2026-09-29 — Replacing the hero paint placeholder with a real mesh

For a morphing mesh, update MeshPoint coordinates instead of easing between
complete Paint values (which crossfades fields). Keep boundary anchors on the
coverage boundary, add paint bleed around a stroked mask, and paint one field
behind the whole Handwriter word rather than giving each letter its own domain.
Only publish a new grid when the visible paint clock advances, so a paused or
unmounted adjective does not continually invalidate the mesh cache.

## 2026-09-30 — Tiled browser surface density

The website paint card exposed the former 1x tiled-scroller override on a 2x display. Tiled surfaces now retain requested screen density, tile sooner at physical budgets, and expose exceptional allocation clamping via `data-pax-resolution-fallback`. Verify backing dimensions: browser viewport emulation can change DPR to 1. See the scrolling/viewports guide for the current contract.

## 2026-10-01 — Rebind measurement from the child list, not an unread signal

The Content Autosizing featurette placed conditional text over the first row.
Its template already used an autosized Stacker, but new children never received
container frames. The measurement rebind effect depended on a unit-valued
received-children notifier without evaluating that notifier. It could remain
dirty after the initial evaluation and suppress later invalidations.

Depend directly on the received-child list, which rebinding actually reads.
This preserves reactive, change-driven measurement rather than polling every
frame or assigning manual positions in the example. Regression coverage toggles
multiple rows repeatedly, checks their ordered frames and stack height, and
resizes a newly inserted row to verify that new measurement dependencies bind.

## 2026-10-01 — Endpoint caps share one reveal position

The Layered Paint card exposed a final round cap snapping into view. GPU stroke
vertices carry normalized centerline advancement, and the round-cap vertices
share their endpoint's advancement. The fragment shader therefore admits the
whole cap when `draw_end` reaches that position. This is not a timeline reset
or gradient discontinuity. A proper swept-cap reveal needs a spatial progress
rule for caps, consistent across stroke widths, compound paths, and mask usage;
changing the easing does not fix it.

For the website study, keep the existing draw/hold/rewind loop but limit the
layered-stroke variant to 97%. Leave the handwriting-mask variant at 100% so
its word remains complete. This is an explicit presentation workaround, not a
renderer fix; the Drawing article records the current GPU limitation.

## 2026-10-11 — Website assets share an example directory

`pax-website/assets` originally symlinked the entire Paxflix asset directory.
Adding a website-only font there would change the example too. The website now
owns its asset directory, with individual symlinks retaining the shared Paxflix
media. Its font lives in `assets/fonts/`, which is bundled on Apple as well as
served on web; `public/` is web-only. Keep Rust `include_bytes!` and native font
consumers on the same source bytes. Apple already registers bundled TTF/OTF
files at startup: select their actual family name, not a browser alias. Check
symlinks and existing chassis registration before adding asset-loading code.

## 2026-10-11 — A successful Wasm build can still have unresolved imports

A fresh website dependency resolution selected Pest 2.9.3, whose standard-library
feature brings in `stacker` and `psm`. The build succeeded, but the browser could
not resolve an `env` module for `rust_psm_stack_pointer`. Inspect both the emitted
JavaScript imports and `WebAssembly.Module.imports` before diagnosing the rendering
code. The website pins its parser family to 2.8.7, the working version, pending
verification of the newer dependency's Wasm path. A build exit code alone is not
a web smoke test.

## 2026-10-11 — Filled font outlines need non-zero winding

Manrope's glyphs contain overlapping contours. They rendered correctly in SVG,
but Pax's GPU paths had small cutouts even while motion was paused. Ordinary
fill tessellation still used Lyon's even-odd default, while mask coverage already
used non-zero winding. Set ordinary fills to non-zero too; regression coverage
checks that overlapping same-direction contours remain filled and reversed
inner contours still make holes. Changing the font outlines to work around the
renderer would conceal the underlying backend inconsistency.

## 2026-10-11 — Apple font parity includes dependency and source parity

A native build registers bundled TTF/OTF assets before drawing. A browser alias
does not name that registered family. Use the font's actual family and ensure
other components do not download competing versions of the same face. The
website now uses one packaged Manrope across its theme, headline and gallery;
Apple font vendoring recognizes existing `assets/` font sources.

A fresh Apple mobile dependency graph selected `libc` 0.2.190, which hides dyld
bindings that `backtrace` 0.3.76 still imports on iOS. The website pins the Apple
`libc` dependency to 0.2.189 until that combination is fixed upstream. A passing
macOS build does not establish iOS or simulator compatibility.

## 2026-10-11 — Stop live reload before switching build targets

Builds of one project share its `.pax/cartridge.partial.rs` and interface staging
directory. Stop the previous preview's CLI before building another target or
mode from that directory so a source edit cannot race the other build's generated
inputs. Template live reload also cannot add fields to an already compiled Rust
component: restart after changing both its template and Rust properties.
Separate worktrees have separate generated directories.

At this branch's baseline, release generation constructs the manifest through a
large Rust expression. The website's generated cartridge exceeds 7 MB; a macOS
release compile spent several minutes in LLVM's SROA/PromoteMemToReg optimization
of the application crate. Use debug builds for visual iteration and distinguish
an incomplete optimized build from a failing application. Apple release runtime
validation still requires a completed build; debug previews do not establish it.

A cached browser preview can keep rendering after its server stops. During a
website release build, unloading that preview restored the compiler from low CPU
utilization to nearly a full core. Check CPU time as well as elapsed time before
diagnosing a compiler hang; temporarily navigate the task's own preview to a blank
page and restore it after the build. Stopping the server alone does not stop the
already loaded Wasm application.

## 2026-10-11 — Fixed lettering need not ship a runtime font engine

The hero's runtime HarfRust/Skrifa path added about 470 KiB gzip to the matched
website release, including an embedded copy of its separately served font.
Import/dependency lists alone do not establish the shipped size: the mounted
headline explicitly called the shaper. A matched baseline retained the same
Cargo dependencies but had no reachable shaping call and shipped no font bytes.

For fixed copy, use the explicit `pax-website/tools/trace-hero` extractor and
check in its output. It retains whole-phrase shaping, cluster ranges, advances,
and exact curves, while the runtime loads ordinary Path elements. HarfRust and
Skrifa live only in that standalone tool, outside both the library workspace and
the website's normal/test dependencies. Its `--check` mode compares the full
generated source against fresh extraction. Keep the native font asset for
selection, and run the check after font or generator changes. Confirm removal
with the normal target-specific Cargo dependency graph and the emitted Wasm,
rather than searching source-drawer strings inside the binary.

Runtime text/API work is deferred to PAX-1036 for Zack-led design. The unpublished
`pax-text` experiment is recoverable from `79a522edbc29614cf39aae70d30fba680f3c4b86`;
keeping the private extraction tool does not expose or commit to that library API.

### Animated descendants can reflow an autosized ancestor despite fixed component dimensions

The website's rotating headline had a fixed height and non-autosizing Groups,
but its parent's autosized Stacker measured the complete descendant layout
hull. Scaling the high-performance word out changed that hull and moved the
paragraph below it (about 15.5 logical pixels in the checked intermediate-width
web view). Turning off autosize on the inner Group would not remove its
descendants from the ancestor's measurement.

Set `layout_role=LayoutRole::Breakout` on the animated word's clipping Frame.
The fixed signboard and normal headline Text nodes then establish the size;
the animated word keeps its clipping, rendering, and native selection behavior.
Do not disable the surrounding paragraph's intrinsic sizing. See
[Layout Role](../layout-responsiveness.md#layout-role).

Path gradients resolve coordinates relative to the path's ink bounds, including
their origin. A baseline-relative negative cap-height offset therefore moves a
gradient above the letters. Use percentage endpoints for a finish that spans
each glyph's visible surface. For readable lit lettering, tune that material's
ambient coefficient rather than adding a layer-wide AmbientLight that changes
unrelated content.

### Moving scoped lights can expose broad redraw invalidation

The website's metallic hero revealed that `LightFrame` scope restricted shader
eligibility while moving its `LightSource` still dirtied every canvas layer.
GPU surfaces and composed-pixel caches also treated any lighting change as a
reason to redraw, including unrelated Paxflix content. Keep lighting-value
invalidation separate from canvas-content invalidation, and derive surface and
capture dependencies from the retained draws' light masks and materials. Include
detached mask-source draws, preserve current uniforms for newly added content,
and invalidate summaries when retained membership changes. Check submission,
capture and geometry counters in addition to frame rate: machine contention can
obscure the cost of unnecessary work.


## User-owned entry and embedded host boundaries (PAX-1015)

- A normal Cargo binary must be selected explicitly alongside the cartridge
  library. Gate the launcher with `standalone`, select it through `default-run`,
  and use `cargo build --lib` for the cartridge. Propagating `standalone` through
  the helper for `pax-kit` features incorrectly asks for `pax-kit/standalone`;
  add the launcher feature at the application package instead.
- Xcode must receive the Cargo executable before its normal signing phase.
  Build the shared SwiftUI host framework first, link the Rust main against it,
  then copy the executable through a declared app build-phase input. A global
  `PRODUCT_BUNDLE_IDENTIFIER` override also renames embedded frameworks and can
  make iOS installation reject duplicate identifiers. Apply app identity through
  a dedicated app-only build setting.
- Swift imports C `size_t` as `Int` on these Apple SDKs. The dynamic macOS
  function types and statically imported iOS functions must agree for copied
  startup/request buffers. Invoke `xcrun --sdk iphonesimulator swiftc` for mobile
  embedding fixtures so xcrun does not inject the macOS SDK into the compiler.
- Temporary SwiftUI sizing views can be destroyed before adopting an engine.
  Their destruction must not close the active mount. Likewise, dismantling a
  rejected second standalone view must not dispose the first view's session.
- The web host previously installed page-level CSS, input listeners, history,
  metadata, cursor and scroll policies. Region mounting requires scoping these
  to the Pax child, preserving host page styles on disposal, and continuing only
  drags that originated inside that child. Do not treat an element argument as
  sufficient proof of embedding.
- `libc` 0.2.190 hides dyld symbols used by `backtrace` 0.3.76 on iOS. Pin the
  mobile dependency to 0.2.189 in the always-present `pax-engine` facade so initial
  Cargo resolution sees it before optional chassis features are selected. Pinning
  only the chassis resolves too late for fresh offline projects. An existing
  ignored example lockfile may need `cargo update -p libc --precise 0.2.189`.
- The macOS bundle executable name contains parentheses. `otool` can interpret
  that pathname as archive-member syntax; inspect a temporary copy under a
  simple filename when verifying the executable's framework linkage.
- Xcode's default active-architecture setting can produce an arm64-only host
  framework beside a universal macOS cartridge. Pass the full Rust target
  architecture set and `ONLY_ACTIVE_ARCH=NO` to both host and app builds.
- Project creation must rewrite target-specific dependency tables as well as
  top-level dependencies, and rename the standalone binary/default-run along
  with the package. Otherwise a fresh project can retain a repository-relative
  host dependency or select a binary under the template's old name.
- Keep native UI delivery separate from acquiring a drawable. A zero-sized
  caller region still needs readiness and queued request delivery. Associate
  each canvas with its own host session so late dismantle cannot close a newly
  mounted replacement or implicitly initialize another cartridge.

## 2026-10-05 — AppKit layout notifications are not scroll input

Calculator's macOS graph opened at the world edge because fractional native
placement caused AppKit to realign its clip origin before the first viewport
layout. The bounds observer published the temporary near-zero origin as a
user scroll, overwriting the authored center and starting the native-scroll
quiet period. Integer-coordinate fixtures missed this; a fractional-placement
fixture reproduces it through an actual NSHostingView/NSScrollView hierarchy.

Suppress native scroll reporting around container geometry and layout writes,
and ignore pre-initialization, empty-viewport, and bounds-size-only
notifications. Preserve the requested offset when content shrinks or when an
explicit position arrives before the geometry from the same patch. Restore
that position within the available bounds after layout, without replaying
stale model fields over actual user scrolling. Cover fractional placement,
content shrink/growth, offset-before-geometry ordering, and native movement in
the Swift tests; verify the mounted example as well.

The same qualification pass exposed the missing macOS/iOS keyboard bridge,
addressed separately above. Click/tap success does not establish keyboard
support; qualify those input paths independently.

## 2026-10-05 — Single-package launchers need transitive framework search paths

The workspace host fixture passed after re-exporting `pax-host` through
`pax-kit`, but Calculator's ordinary single-package native build failed with
`framework 'PaxHost' not found`. Cargo builds the package's library alongside
its binary, including its `cdylib` target. The arguments after `cargo rustc --`
reach only the selected binary, while the enabled host dependency also adds
framework linkage to that intermediate library build. Pass the framework
directory through `PAX_HOST_FRAMEWORK_DIR` to `pax-host`'s build script and emit
`cargo:rustc-link-search` there, so every consumer gets the path. Keep the
cartridge build separate with `standalone` disabled; inspect the packaged
cartridge to ensure it does not acquire a dependency on `PaxHost`.

Qualification must include both an ordinary package with a library and binary,
and a workspace launcher selecting another UI package. Neither layout alone
establishes the other. Build success also does not establish presentation:
Calculator's macOS app launched and evaluated arithmetic, but its initial graph
opened at the edge of the scrollable world in both standalone and embedded
hosts. The iPhone and web builds showed a centered graph. That native scroller
finding remains separate from the corrected launcher link failure.

### Explicit cartridges and workspace UI packages (PAX-1015 phase two)

- Public declarations should explain the author's intent before introducing
  packaging terminology. `runnable!` selects the root and initializer of a UI
  that a host can launch or embed; cartridge names its compiled representation
  and linkage boundary. The declaration itself does not initialize or mount it.
- A separate implementation crate need not become another application import.
  Re-export the native SDK as `pax_kit::host`, and `init`/`init_with` at the facade
  root, with `standalone` enabling `pax-kit/host`. Keep this feature off in the UI
  library build so the cartridge does not depend on its own hosting framework.
- Repeating platform `cfg` branches in every launcher obscures the normal Rust
  entry. The binary already requires `standalone`; put unsupported-host-target
  diagnostics in `pax-kit` and let common examples use a single `main`.
- Cargo metadata's `root_package()` is absent in a virtual workspace even when a
  package manifest was selected. Match its canonical manifest path instead. Use
  Cargo's reported target directory; sibling packages share it and callers may
  set `CARGO_TARGET_DIR`.
- The host's `.pax/` directory does not identify the UI library when they are
  separate packages. Pass the selected library directory explicitly to macros,
  track that environment in Cargo fingerprints, and suppress cartridge exports
  during launcher compilation. Reusing a cached UI library as a dependency must
  not reuse its standalone entrypoints.
- A source definition path may go through a private module. Static discovery
  now selects a deterministic public re-export when generated code needs one.
  Transitive components must also be exposed through a directly accessible
  library's public API, or be direct dependencies of the selected cartridge.
- Source-module identity for implicit lifecycle lookup must use the dependency's
  library namespace. Hard-coding `crate::` silently omitted dependency mount
  handlers, even when explicit event handlers worked.
- Watch the sibling UI and referenced template/asset directories, not only the
  host directory. The rich manifest retains actual template paths; the release
  program retains the same selected root and component roles.
- The existing app-icon PNG is grayscale. Using it as native scene image data
  exposed an existing CGContext decoding failure. The layout fixture uses the
  existing RGBA Space Game assets to exercise portable asset packaging; grayscale
  scene-image decoding remains a separate renderer issue.

## 2026-10-05 — Native keys need focus ownership and a cross-language wire test

Calculator already handled `@key_down`, but Apple hosts did not forward hardware
keys. The bridge now uses the canvas's native first-responder methods. Standalone
apps acquire initial focus; embedded mounts preserve host focus until a canvas
click/tap. Do not use process-wide event monitors: they can steal keys from host
editors and Pax native controls. Release held keys on focus loss and teardown,
and return Rust's default-prevention result to the native responder chain.

A Swift-only encode/decode test missed a real crash on shifted keys. Swift's
typed string vector decoded as `Key` elements in Rust flexbuffers 2, while
Serde's modifier enums require `String` elements. Emit an untyped vector with
explicit strings. `PaxKeyboardTests` and `native_keyboard_probe` share a fixture
that passes the actual Swift bytes through a built Calculator cartridge; run
the probe against both debug and baked release builds. Test native host focus
and actual keyboard arithmetic as well as the isolated event state machine.

## 2026-10-05 — Cargo target selection does not isolate features or resource lookup

The phase-three graph prototype showed that an ordinary single-package
`cargo run` builds the package's `cdylib` alongside its implicit `rlib`
dependency, but `standalone` features apply to that library as well as the
binary. A binary's `required-features` is a selection gate, not a private
feature set. Binary-specific link directives can keep `PaxHost` out of the
cartridge's native dependencies; they do not undo Cargo feature unification.
The phase-three spec records the resulting design decision. Do not infer
strict feature isolation from a successful build or from absent host symbols.

The native proof also failed initially because font/resource discovery
assumed `Bundle.main` contained the Swift assets bundle. A bare Cargo binary
has no application resource directory. Swift's generated `Bundle.module`
lookup worked after copying the assets bundle into the owning host framework;
exposing the accessor alone was insufficient. Test direct execution from
another working directory with Cargo/Pax/dyld launch variables removed, and
distinguish a root-mount callback from visual or input validation. These are
prototype findings; the released/CLI workflow has not changed.

### Cargo and macro source generation (PAX-1015 phase three)

- A library and binary in one Cargo package share feature requests. Gating the
  binary with `required-features` does not isolate its dependency features.
  Preserve native artifact/linkage and runtime ownership explicitly; inspect
  symbols and loaded libraries instead of treating a Cargo feature as a mount.
- Static analysis of source files inside a procedural macro cannot assume that
  compiler-owned spans are offsets into those files. The no-build-script proof
  failed that way for inline Pax. A scoped source-parser backend preserved the
  actual source, including whitespace; do not repair this by stringifying tokens
  or moving compiler token streams to another thread.
- Multiple runnable declarations should feed one per-compilation program catalog.
  Resolve aliases before deduplicating component definitions, retain initializer
  choices separately, and require explicit root selection when ambiguous. A
  runnable component used as a child must not run its application initializer.
- Cargo may compile different dependency units for `build`, `check`, test
  harnesses and profiles. The generic native SDK can be cached within those
  units, but promising one SDK build across all Cargo modes is inaccurate.
- Source discovery currently traverses disabled `#[cfg]` branches too. A
  macro-build stress case with `#[cfg(any())]` produced a duplicate-component
  error even though Rust would remove the item. Moving discovery into a macro
  does not fix configuration handling; qualify against the actual Cargo target,
  features and custom cfgs before switching the default frontend.
- A proc macro can ask the owning compiler to select literal cfg answers through
  conditional continuation invocations. A dependency must export answers it
  selected while compiling; leaving predicates inside an exported macro would
  evaluate them against the consumer. The real-Pax configuration fixture proves
  differing app/dependency features, dependency build-script cfgs, wasm, release
  and the app-test/dependency-library distinction without an app build script.
- An automatic configuration-helper emitter cannot be the first textual Pax
  declaration: that declaration may be disabled. Select a deterministic active
  source site after configuration. Cache parsed source and completed discovery;
  repeated declarations still have callback overhead that needs qualification
  against the complete standard library.
- `#[pax]` and the manifest scanner must agree on conditional fields. Filtering
  the source manifest alone leaves attribute-generated Rust referring to fields
  rustc removes. The fixture feeds the same configured definition to both.
- Expanding `cfg_attr` synthesizes the outer attribute span. Recover inline Pax
  from its original parenthesized argument group, not the synthetic `#[...]`
  span or token-string formatting, to preserve whitespace and Unicode.
- Host-side code generation and target-side rich manifest loading need matching
  JSON representation features. Cargo resolver 2 does not inherit a proc macro's
  `pax-manifest/json` feature into the target library. The fixture enables both
  `pax-engine/json` and `pax-manifest/json` explicitly for rich reconstruction;
  baked constructors are independently exercised.

### Cargo-native rollout: configuration helpers and broad example checks

Generated macro helper names must be unique to their owning library. A shared
`__pax_source_config` name was reexported by facade/component globs and became
ambiguous when a runnable example depended on another runnable example. Use a
stable helper identity derived from the library source entry, and preserve the
requesting crate's syntax context through callbacks. Export dependency snapshots
from their owning compilation rather than re-evaluating their cfgs in consumers.

When reconstructing a configured item from source, do not replay attributes that
rustc already expanded before `#[pax]` (for example, a preceding `derive`). Keep
the remaining compiler-input attributes while selecting fields from the source
snapshot. Otherwise ordinary derives gain duplicate trait implementations.

Turning the examples into real Cargo targets catches Rust/template drift that
proxy launchers hid: color-picker's callback needed `LocalProperty<Color>`,
store-ownership still used `Stroke.color`, and mouse-animation used a bare Color
for Paint and the removed `PathElement::close()` constructor. Keep the full
example target check alongside targeted native/web launch tests.

A failed re-export lookup can mean the owning dependency's configuration has
not arrived yet, rather than that the name is absent. Preserve the typed
missing-source request through optional name-resolution fallbacks and contextual
errors. The production fixture combines a renamed dependency, a transitive
private-module re-export and a custom library path to exercise this distinction.

Cargo ignores profiles inside workspace members. Moving the examples into the
main workspace therefore requires moving their selective debug optimization
policy to the workspace root, and restoring it when exporting CLI-created
standalone projects. Keep both forms under the bundled-example regression test.

A `runnable!` declaration may live in a library root containing only module
exports, without `use pax_kit::*`. Generated cartridge modules must import their
own engine/API traits and facade standard-library alias. The separate-host
fixture deliberately keeps that lean library root, a renamed UI dependency,
and a custom library target name; adding a user prelude to hide generated-code
imports would miss the integration bug.

`cargo check --target wasm32-unknown-unknown` cannot catch duplicate wasm exports.
Slot Particles reuses runnable Fireworks: a real wasm link first caught the native
program-identity symbol, then wasm-bindgen caught both libraries exporting
`init_web`. Keep native C identity exports native-only, namespace each web adapter
by Cargo package/target identity, and let packaging alias the explicitly selected
adapter to `init_web`. Gating on unified features or primary-package status would
not handle multiple selected workspace packages reliably.

The same Slot Particles smoke test exposed a pre-existing retained GPU scene
limit: `TransformArena` has 480 slots including reserved identity slot zero.
Exhaustion logs an error and aliases overflowing entries onto that identity slot;
the WGSL uniform array is fixed at the same size to fit low adapter limits. A
proper fix needs allocation/binding work, not fewer particles or a larger magic
number. Follow-up [PAX-1028](https://linear.app/paxdev/issue/PAX-1028/remove-the-retained-gpu-scenes-480-transform-ceiling)
records reproduction, source evidence and the adjacent clip-arena audit. Zack
deferred this renderer work from the Cargo-native rollout.

Compiling an iOS target alone does not assemble a runnable app. A Cargo simulator
runner prototype successfully packaged and launched Increment, including a tap
that changed its counter. Its separate cartridge matched the executable and
packaging preserved the original Cargo artifact hashes. It was then backed out
at Zack's direction: a Cargo configuration forwarding to an installed `pax-cli`
adds setup without removing the tool dependency. Keep `cargo run` for macOS and
`pax-cli run --target web|ios|ipados` for the other existing launch workflows.

Cargo does not load a dependency's `.cargo/config.toml` or install its binary
targets on `PATH`. Shipping a helper inside `pax-kit` therefore cannot activate a
runner by itself. A project-local tooling crate could avoid a global install, but
still needs explicit configuration and more scaffolding. A diagnostic probe also
showed that runners receive the executable and app arguments, not `TARGET` or
`PROFILE`; any future artifact-consuming packager needs explicit metadata and
must preserve profile, target, feature/cfg selection, and Cargo example identity.

The prototype exposed an existing packaging issue: Apple install-name edits
mutate Cargo's original dylib. Future packaging should edit staged copies before
relocation/signing. Production distribution and App Store qualification are
tracked in [PAX-1029](https://linear.app/paxdev/issue/PAX-1029/support-cargo-driven-production-distribution-and-validate-apple).

Dependency build scripts are not reliable usage hooks: Cargo caching suppresses
warm-build invocations, while transitive dependencies may build without active
Pax development. CLI telemetry remains limited to the CLI cohort. Prefer explicit
opt-in developer-tool events and active-day/return metrics if extending coverage;
do not introduce network activity into macro expansion or compile-time hooks.


### Development launch and wrapper argument boundaries (2026-10-06)

`wasm-pack` starts forwarding trailing Cargo switches after an unrecognized Cargo
option such as `--no-default-features`. Appending wasm-pack's `--dev` afterward
sent it to `cargo build`, which rejected it. Build wrapper/profile options first,
then append Cargo feature switches. The production feature boundary now expands
application defaults explicitly, omits the direct designtime default, and checks
normal target dependencies for transitive activation.

A default development entry must distinguish workstation orchestration from a
native application that also enables designtime. Carry the application role as a
tracked compile-time input (`PAX_BUILD_ROLE`) rather than inferring it from
`designtime` or debug assertions. Explicit `init_with!` stays in-process.

## 2026-10-07 — Narrow package selection does not make the development launcher cheap

The `-p pax-logo` workaround removes unrelated examples, but leaves a substantial
workstation build. A Cargo unit-graph comparison between branch base
`14b6f8d6a` and `7c11d31eb`, using the same dependency lockfile and installed
nightly Cargo for planning, found the following on an Apple Silicon Mac:

| Build selection | Package/version identities | Rust compilation units |
| --- | ---: | ---: |
| Base: example's default native Cargo wrapper | 148 | 279 |
| Base: wrapper with `pax-kit/designtime` | 279 | 549 |
| Before performance pass: `cargo build -p pax-logo` defaults | 458 | 768 |
| Before performance pass: `cargo build --example pax-logo` | 509 | 818 |
| Base: development Wasm library | 283 | 503 |
| Before performance pass: development Wasm library | 301 | 525 |

Units count rustc invocations, including compilation of build scripts, but omit
running those scripts. They are build plans, not measured cold-build durations;
do not sum the workstation and Wasm rows as though caches cannot be shared.
The old wrapper invoked an external CLI, whose installation/compilation cost is
outside its row. Default designtime and moving the compiler/packager into the
application materially change this comparison: `pax-kit/designtime` now brings
in `pax-compiler` on workstations. macOS also selects its native chassis before
the development launcher can choose web. That implementation added a target `cargo check`
to obtain the compiler-selected manifest before its actual target build;
previously manifest extraction was static analysis inside the CLI.

Duplicate build/runtime dependencies amplified these costs. Before the performance
pass, the workstation built `pax-codegen`, `pax-runtime`,
`pax-designtime`, wgpu and Naga at both optimization levels 0 and 1. Profile
alignment alone reduced the planned compilation count from 768 to 612 in a
diagnostic experiment; that does not establish a wall-time or runtime-performance
improvement and is not a reason to change profiles without timing them.

An isolated source-copy experiment removed the unused direct `pax-runtime`
dependency from `pax-macro`, retaining `designtime` as a macro configuration
feature. The runtime names in macro templates are emitted through the consumer's
engine facade and do not require this host dependency. All eight macro tests
passed; workstation compilation units fell from 768 to 648, and development
Wasm units from 525 to 393. The Wasm build no longer needed a host GPU renderer.
This dependency predates the branch. The follow-up implementation and validation
are recorded below.

Qualify complete cold and warm builds in separate target directories, recording
launcher compilation, manifest extraction, target compilation and packaging
separately. Preserve the compiler-selected cfg/feature handoff; returning to
configuration-blind scanning would trade away correctness. Functional smoke
tests and Wasm payload measurements do not qualify build latency.

### Follow-up performance pass

Removed the unused runtime edge from both `pax-macro` and `pax-compiler` (the
latter only imported Serde's `Deserialize` through it). The compiler-handoff
fixture now guards both tools' graphs against pulling in `pax-runtime`,
`pax-gpu`, wgpu or Naga. The native launcher still needs its application runtime,
so its unique package count remains 458, while Rust compilation units fall to
648. Development Wasm needs 254 package versions and 393 compilation units.

Web packaging now invokes wasm-pack once, reads the manifest produced by that
same compilation, and then prepares routes, assets, and served output. It does
not run a preliminary check or build. Apple manifest preparation uses a real
target build, allowing subsequent native packaging to reuse its artifacts.
Compiler-selected source, dependency features, custom cfgs, and release baking
remain authoritative. The public application bootstrap is unchanged.

Warm-build diagnostics exposed generated metadata invalidating itself. Cargo
reported the manifest sidecar as newer than the invocation producing it, and
JSON comparisons showed both arbitrary object-key order and shuffled expression
dependency lists. Serialize the sidecar with sorted object keys; retain first
occurrence order when deduplicating expression and template dependencies.
Template discovery also orders dependency asset directories, so its iteration
must be stable. Preserve meaningful array order and values. The release binary
schema is unchanged; manifest/ProgramIR round trips verify dependency order.
The sidecar remains a tracked input so deleting it causes regeneration. Its
initial creation can require one additional app compilation on the first warm
run; repeated unchanged runs should then perform no Rust compilation.

A trial aligning build dependencies at optimization level 1 reduced the planned
native compilation count further, but did not improve measured cold latency.
Keep the existing profiles: dependencies at level 1, build dependencies and app
code at level 0. Changing profiles merely to improve a crate count is misleading.

Measured `cargo run --timings -p pax-logo -- pax --target=web --build` on this
Apple Silicon workstation with Rust/Cargo 1.93.0, the same dependency lockfile,
separate empty Rust target directories, and cached registry/platform tools:

| Measurement | Before pass (`7c11d31eb`) | After pass |
| --- | ---: | ---: |
| Complete cold build | 336.64 s | 269.15 s |
| First unchanged rerun | 12.55 s | 11.12 s |
| Second unchanged rerun | Not measured | 5.11 s |

The final cold run spent about 154 seconds building the launcher and 113 seconds
in Cargo/wasm-pack for the destination. The second unchanged rerun performed no
Rust compilation. These are individual workstation measurements, not statistical
claims or download/installation timings. The cold reduction is about 20%; it
does not make the remaining four-and-a-half-minute first build lightweight.
Use `scripts/benchmark-cargo-web.py` for repeatable cold/first-warm/steady-warm
trials without deleting an existing development cache.

Validation includes 326 focused tests across language, macro, codegen, compiler,
and manifest crates; the native/wasm/release/custom-cfg configuration-handoff
matrix; independent native engine initialization; and release web packaging via
the CLI. The language suite's older evaluation fixtures needed explicit property
graph ownership in their test helper; production evaluation was not changed.
These checks do not claim a new full simulator, device, or other-workstation
qualification pass. Remaining first-touch cost includes compiling the native
engine and the packager into the workstation launcher before it selects web.

## 2026-10-07 — Repository example builds and browser payload are different graphs

The root package currently lists 49 example libraries as dev dependencies.
`cargo run --example pax-logo -- pax --target=web` therefore has a much broader
workstation build than `cargo run -p pax-logo -- pax --target=web`. Use package
selection while investigating one example. The example wrappers need a narrower
dependency arrangement; their unrelated libraries are not all bundled into the
selected browser application.

Audit target dependencies with `cargo tree --target wasm32-unknown-unknown
--edges normal,no-proc-macro`, separately from workstation/proc-macro builds.
Comparing the branch base (`14b6f8d6a`) with `7c11d31eb` showed no new
`pax-compiler`, `pax-codegen`, Actix, or Cargo-metadata browser dependencies.
Development already enabled parsing/code-generation helpers through
`pax-designtime`. However, preserving the application's `standalone` default
also introduces `pax-host`, `include_dir`, and `serde_json` into release Wasm's
compilation graph. Those native packaging dependencies should be gated more
narrowly, even when their unused code is removed at link time.

A controlled current-branch `pax-logo` release comparison, after wasm-bindgen
0.2.129 with the name section removed and before optional wasm-opt, measured
3,239,469 bytes with `standalone` versus 3,239,472 without it. Gzip sizes were
946,923 and 946,866 bytes respectively. The symbol-preserving release input
contained no `pax_host`, `serde_json`, `include_dir`, `pax_compiler`, or
`pax_codegen` names. This qualifies the hosting-feature concern; it is not a
complete pre-branch/current binary-size benchmark. Keep that distinction when
reporting the result.

## 2026-10-11 — Example moves cross discovery and cleanup boundaries

Moving secondary packages into `examples/fixtures` requires more than Cargo
path updates: immediate-child discovery silently drops them from CLI docs,
short hosted names must resolve separately from source paths, and the old
`examples/*/.pax/` ignore rule prevents cleanup from deleting nested build
artifacts. Discovery now covers exactly the curated and fixture levels and
rejects duplicate short names. The ignore rule covers both levels; cleanup
retains its ignored-directory and tracked-file safeguards. The scaffold helper
registers fixture Cargo targets explicitly and supports an explicit showcase
location. Audit extensionless `pax` helpers too: their old three-level CLI path
was still present in curated packages after the earlier flattening. Verify both
source bundles and use the CLI/package tests after moves. Docs embed manifests
must distinguish hosted names from source paths; validate the actual release
builds as well as mocked commands. Include the extracted code-generation/host
crates and Cargo configuration in the embed cache recipe so initialization
refactors cannot leave a stale app behind an up-to-date source tab.

## 2026-10-10 — Dependency macros must not resolve their enclosing workspace

A newly created project on Ubuntu failed while compiling source-linked `pax-std`:
full offline `cargo metadata` inside its macro demanded an uncached dependency of
an unrelated monorepo member. Warm workstation caches hid the assumption. The
fix uses no-deps package metadata and compiled library helpers to hand off source
identity, actual cfg facts, and dependency links. Native initialization uses the
same style of handoff for its program recipe. No application build script or
extra dependency fetching is required.

A fixed generic helper name collides through glob re-exports, so helpers use
package-qualified names, with aliases for custom library target names. The
consumer's Rust import selects the compiled dependency. Track both
`CARGO_MANIFEST_DIR` and `Cargo.toml` in emitted code: helpers contain absolute
source paths, and Cargo can otherwise reuse them after a source checkout moves
with preserved mtimes and a shared target directory. Compare canonical paths
when matching workspace selections, including macOS `/var` and `/private/var`.

The regression deliberately gives a library's unrelated workspace member an
unresolvable dependency, then compiles an external consumer with ordinary Cargo.
It also relocates the library sources without clearing the target cache. Keep
these controls cold with respect to the unrelated dependency; fetching the whole
workspace is not evidence that this failure has been fixed. See the
[qualification record](PAX-1015-main-entry-spec.md#resolved-cold-dependency-workspace-metadata).

## 2026-10-09 — Initialization should not also be a workstation build tool

The embedded development launcher made native `main` ambiguous: code before
`init!` ran on the workstation for web/mobile launches, and could run again when
the executable served as a native reload worker. It also pulled the packager
into the application dependency graph. The rollback restores in-process native
initialization, removes the `pax-kit` → `pax-compiler` edge, and uses the CLI for
serving, packaging and deployment. Native SDK assembly happens during Cargo
compilation, not during initialization. The macro/compiler split and single-pass
target builds remain. The older measurements below describe the superseded
embedded-launcher architecture; the benchmark script now excludes CLI installation.

The post-rollback Pax Logo measurement was 137.400s Rust-cold, 8.966s first-warm,
and 3.201s steady-warm, with no Rust compilation on the final run. The first warm
run rebuilt only the application after initial sidecar creation. See the
[qualification record](PAX-1015-main-entry-spec.md#final-workstation-and-timing-qualification-2026-10-10)
for the cache/toolchain conditions; these numbers exclude CLI installation and
must not be compared directly with the old combined launcher/target totals.

## 2026-10-12 — Fresh dependency resolution can compile but fail to mount

The fresh Pax Logo starter resolved Pest 2.9.3, while the repository lockfile
retained 2.9.2. Both compiled, but 2.9.3 enables `stacker` and pins `psm` 0.1.24
through its `std` feature. Its Wasm artifact imported the unresolved function
`env.rust_psm_stack_pointer`; wasm-bindgen emitted an `env` module import and
the browser rejected the application before mounting. `pax-language` now pins
Pest and its derive crate to the existing tested 2.9.2 pair. Revisit the pin when
upstream's Wasm stack-checking path is fixed. A fresh external starter must be
mounted in a browser, not merely compiled; a successful Cargo or wasm-pack exit
does not prove its imports resolve. The package smoke check now rejects this
unresolved `env` import in the generated bindings.

## 2026-10-12 — Reusable branding should not expand the starter

The logo harness and Living Quilt carried duplicate animation geometry, while
the website depended on the harness package just to mount its logo. Moving the
animated and settled components into `pax-std` keeps a fresh Pax Logo starter to
three source files and lets consumers use the existing `pax_kit` facade. Keep
playback ownership at the call site: Living Quilt disables the shared logo’s
click replay because its card already owns replay, and uses the settled
mask-free component after the entrance. Update View Source includes, topology
anchors, both bundled example snapshots, and create/package smoke assertions
when moving a component out of a bundled starter.

Replay must reset and enqueue its transition through the same local property
view. A shared `Property::set` is imported on a later graph turn; mixing it
with an immediate `local().ease_to` can retain the old starting position. The
logo and its harness now reset locally. Animation unit tests need an entered
`PropertyGraph`, `LocalProperty` clocks, and effect draining before reading the
published progress, matching the engine’s frame settlement.

## 2026-10-12 — Shared web caches and Windows source prerequisites

An external starter and a monorepo fixture can select different dependency
graphs while sharing `CARGO_TARGET_DIR`. In the Ubuntu VM, building the starter,
then Space Game, then the starter again reused an incompatible
`libpax_chassis_web.rlib`: Cargo reported the chassis as fresh, but Rust rejected
apparently identical `JsValue`, `AppOptions`, and runtime types. The chassis's
combined `cdylib`/`rlib` declaration produced an unhashed dependency artifact.
The application cartridge already owns the final Wasm binary, so the web chassis
now emits only an `rlib`, whose filename includes Cargo's dependency identity.
The same starter → fixture → starter sequence then passed without clearing the
cache. Retain cross-workspace cache switches in qualification; a fresh target
directory can conceal this failure.

The Windows source build also needs Git for Windows' `bin` directory on PATH,
in addition to `cmd`: the source checkout rebuilds its JavaScript interface by
invoking `bash`. The first-touch source harness now includes that directory.
Published CLI packages contain the interface bundle already. Do not generalize
source-checkout Node/Bash requirements to an ordinary published CLI installation.

Release validation must also use the same working directory as compilation.
`cargo tree --manifest-path` does not discover the selected project's
`.cargo/config.toml` from another directory. A source-patched starter therefore
built from its own directory but failed the CLI's release feature preflight when
selected with `--path`. Run that resolver in the selected project directory.
The regression uses a project-local patch and verifies that the guard still
detects transitive development features through the patched dependency.

## 2026-10-13 — Website marquee startup over-allocates projected slots

Investigating startup duplicate-slot warnings in `pax-website`: expected a
viewport-sized card window with unique projections. Temporary geometry logging
on current main plus the website branch observed mount with card width 0,
stride 25, viewport 1280, and 43 projected cards; the next geometry pass read
width 350 and stride 374 but retained 58 cells. The high-water window policy
never shrinks that initial allocation. Live scene inspection confirmed 43
populated slots and 15 empty duplicate slots, versus 10 cells needed at the
settled size. All captured duplicate-slot warnings identified `MarqueeCarousel`.
The parent publishes card width from its mount handler, while shared `Property`
writes reach local template bindings at a settlement boundary. Fixed with a
valid default width, local-view layout publication, an unready-geometry guard,
and a window cap that prevents duplicate projected indices. Runtime warnings
remain intact; regression tests cover startup, resize, loop movement, and
short/changing content. Website-specific guidance lives in its README.

The same smoke test produced one `pax-tile-window-escape` warning after a
deliberate 18-page horizontal scroll jump (about 23,000px). Card rendering
recovered and slot counts stayed valid. This renderer warm-window diagnostic
is separate from the projection fix; ordinary movement produced no slot
warnings. Reproduce with the desktop feature rail before attributing it to
carousel allocation or broadening this fix into renderer work.
- 2026-10-07 — PAX-839 inspection at `518929f22`: tried the documented
  `pax-cli dev --help` before startup profiling. Expected the current developer
  commands; `/Users/zack/.cargo/bin/pax-cli` reports `pax 0.38.2` and rejects
  `dev`, while this checkout uses 0.39.0. Use a CLI built from this checkout for
  subsequent measurements; the inherited PATH binary is not a valid baseline
  tool. No performance conclusion was drawn from it.

- 2026-10-09 — PAX-839 release-web experiment at `518929f22`, Chrome 154,
  1440×900/DPR 1: replacing Paxflix card reveals with final `x=16px opacity=1`
  should preserve visible captions. After scrolling the website root to 1100
  and the embedded catalogs to 460, captions were missing despite matching
  visible DOM bounds, opacity, and text styles. Changing a missing title's DOM
  opacity to 0.999 made it appear; removing container clip paths restored the
  remaining captions. This suggests a clipping/repaint interaction, not a
  confirmed cause. Repro,
  temporary variant patch, and screenshots: `/tmp/pax839-experiment`; findings
  recorded on PAX-839. Experimental source changes were restored.

- 2026-10-09 — PAX-839 startup profiling at `518929f22`: opening the unmodified
  release website with `?pax_scroll_perf=1&pax_log=warn` in Chrome 154 should
  initialize without duplicate slot-consumption warnings. Each of three warm
  runs emitted 146 `slot index ... was already consumed by an earlier slot
  projection; rendering no slot child` warnings. The instrumented builds emitted
  the same warnings; they are not introduced by the profiling probes. No visual
  consequence or cause has been established. Raw logs and profiles:
  `/tmp/pax839-breakdown/compare-*-normal.json`; related investigation: PAX-839.

- 2026-10-10 — PAX-839 warm release-web A/B profiling at `518929f22`,
  Chrome 154: expected comparable startup loads after a 15-second compiler-free
  interval. Other worktrees started compilers during a retained load; another
  baseline took 4.98s versus 3.11–3.31s without observed compiler overlap while
  renderers and simulators were active. Exclude pairs with sampled compiler
  overlap, retain the other outlier, and qualify paired timing estimates; use
  deterministic allocation counts and paused visual comparisons as separate
  evidence. Captures and CPU samples: `/tmp/pax839-fresh-defaults-20261010`.

- 2026-10-11 — PAX-839 temporary web profiling: added a numeric-keyed
  `BTreeMap<usize, u64>` to `tick_profiled` diagnostics, expecting a serialized
  histogram. The chassis `js_value_serde` accepts only string map keys and
  silently returns `null` for the entire result on serialization failure,
  interrupting the browser tick consumer. Stringifying histogram keys fixed
  the diagnostic capture; all probes were subsequently removed. Evidence:
  `/tmp/pax839-component-defaults-20261010/serializer-failed-capture`; consider
  surfacing the serializer error when adding future developer diagnostics.

- 2026-10-11 — Running all web interface Node tests also launches Rust surface-policy fixture builds from `tests/shared-surface-policy.mjs`. Expected a small JS validation pass; observed a second Cargo profile build and unrestricted Rust parallelism competing with the website release build. Repro: `node --test pax-compiler/files/interfaces/web/tests/*.test.mjs` without the Rust environment used by the main checks. Workaround: pass `CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_NET_OFFLINE=true` to the Node test command as well.

- 2026-10-11 — PAX-839 release-constructor validation: `cargo test -p pax-manifest --features compiler` passes the manifest/binary/mesh suites, but three parser integration expectations fail in untouched parsing code. The two RouteMetadata rejection tests encounter `Router expects direct route branch children` before their expected metadata diagnostic; `test_type_qualified_object_constructor_remains_supported` receives a declarative `Block` instead of its expected value. Evidence: `/tmp/pax839-startup-ux-20261011/manifest-tests.log`. These parser expectations were left unchanged; generated website manifest equivalence is checked separately.

- 2026-10-12 — PAX-839 SettingsPatch A/B captures again overlapped other worktree compilers: an enabled run took 5.92s for tick one versus 3.08s in the preceding enabled run, despite identical binding/allocation counts. Elapsed timing alone cannot establish the optimization's size. Repro and process samples: `/tmp/pax839-settings-patch-20261012/capture-probe-*.json`. Workaround: collect Chrome Performance metrics with `timeDomain=threadTicks` at `console.timeStamp` checkpoints, alongside elapsed timing and deterministic counts; report renderer CPU separately from first-paint latency.

- 2026-10-12 — PAX-839 startup timing still varies after waiting for compilers: unchanged baseline renderer CPU ranged from 9.16–11.48s while a simulator, Windows VM, and other animated previews were active. Thread CPU excludes descheduling but does not normalize execution speed under shared workstation load. Evidence: `/tmp/pax839-demand-settings-20261012/capture-*.json` includes process samples and CDP thread-time checkpoints. Workaround: use contemporaneous, balanced baseline/candidate pairs, report ranges, and keep first-paint latency separate; the candidate improved all five pairs.

- 2026-10-12 — PAX-839 preparation validation: a retained typed reader depending
  on a container property returned its old nested value after closing a raw
  `BindingScope`, editing the nested shared property, and resuming the scope.
  Expected 3; observed 2. The new `prepare()` path and the original
  `get_as_pax_value()` call both reproduce this; it is not a regression from
  common-property preparation. This probe does not exercise a full node remount
  with rebinding. Reproducer and both test logs:
  `/tmp/pax839-prepare-common-20261012/nested-test-candidate.rs`,
  `api-tests.log`, and `nested-legacy-check.log`. Compound preparation remains
  unchanged in this slice; investigate the retained reader's resume invalidation
  separately.

- 2026-10-12 — Web bootstrap validation: `npx tsc --noEmit` reports existing interface errors (native patch types, native text CSS types, touch inference, and cartridge/window declarations). Expected a clean full typecheck; the before/after compiler-host comparison returns identical diagnostics with the pre-task bootstrap sources restored virtually. Evidence: `/tmp/pax839-resumable-implementation/{typescript.log,type-comparison.json}`. Focused scheduler/native-text tests and the release esbuild remain available; a passing bundle is not a passing full typecheck.

- 2026-10-14 — PAX-839 browser validation: the active worktree's `target`,
  `pax-website/target`, and `pax-website/.pax` directories disappeared between
  completed release captures and the next capture, while its preview server was
  still running. Expected the active preview/build caches to remain available;
  observed 404s for the page and a full rebuild required. Tracked sources were
  intact. No cleanup command was run in this task. Evidence:
  `/tmp/pax839-checkpoint-20261014/checkpoint-wide-4-stuck.json` and the 50561
  server log around 12:31 as displayed; cause not established.

- 2026-10-14 — PAX-839 screenshot previews: the web canvas `Image` path renders
  native sRGB JPEG captures substantially darker/posterized than a browser
  `<img>` showing the identical file. Expected matching colors; side-by-side
  evidence: `/tmp/paxflix-showcase-20261014/image-comparison.png`. Both JPEGs
  report sRGB, RGB, 8-bit samples. Using the existing `NativeImage` component
  for the website's static previews; the underlying canvas color discrepancy
  remains to investigate.

- 2026-10-14 — Native screenshot capture: `pax-cli run --path examples/paxflix
  --target ios --ios-device simulator:AD4A9212-1BF5-4B6C-AA46-5809580D6947
  --libdev` launched a rendered, interactive app, but `dev list` remained empty
  and `dev look --path examples/paxflix` reported no live session. Expected
  the same discoverable capture session as macOS. Log: `/tmp/paxflix-native-ios.log`;
  used `xcrun simctl io <uuid> screenshot --type=jpeg <absolute-path>` instead.

- 2026-10-15 — PAX-839 typography review in the in-app browser at 1280×720:
  after scrolling the homepage to the calculator, its native text was visible
  but its canvas shell/keys/graph were absent. Expected the complete device.
  Reproduced in both the unchanged pre-font release on port 50563 and the
  candidate on 50561 (both `?pax_mount_yield=1`), and in debug on 50562. The
  baseline/candidate browser screenshots are in this chat; build and baseline
  copies are under `/tmp/pax-fonts-20261015/`. This is not introduced by the
  font changes; relationship to the deferred clipping/rendering issues remains
  unestablished.

- 2026-10-15 — PAX-839 overlapping native hit regions: scrolling over the
  tablet showcase's upper area moved the outer page. `elementsFromPoint`
  identified the rear macOS `NativeImage` even where its SVG occlusion mask
  hid it. CSS masks affect painting, not browser hit-testing. NativeImage now
  leaves DOM input to the underlying scroller while Pax routes its events;
  upper/middle/lower tablet wheel checks pass. Other native widgets that need
  browser input (selectable text, controls, iframes) have not been audited for
  this same occlusion/hit-region mismatch.

- 2026-10-25 — PAX-1047 drawing-pad resize: a horizontal Scroller bound to a
  pixel offset can drift when its viewport and content width change together.
  At 390px browser width, offset 858px represented logical column 168; after
  restoring the desktop viewport (512px pad width), the position settled at
  1204px instead of the requested 1536px. Ink coordinates remained correct and
  moving the position bar to 1536px revealed the intact mark. Repro: draw in the
  fourth pad viewport, then resize with the editor open. The example attempts
  to preserve the logical column on resize; native scroll feedback during
  layout needs investigation. No engine workaround added.

- 2026-10-25 — PAX-1047 LED glow resource tracing: shader pipelines and pass
  textures reuse correctly (zero allocations across 667 steady animation
  frames), but the changing batched LED paths recreate up to five vector GPU
  resources per redraw. A 300-frame comparison with the shader removed still
  reported 1336 vector resource creates/updates/recreates, so this is present
  in the original path animation as well. Evidence: local `pax_log=trace`
  captures `/private/tmp/pax1047-glow-trace-steady.json` and
  `/private/tmp/pax1047-no-glow-trace.json`. Consider capacity reuse for changing
  path geometry separately; no capacity-reuse change made.

- 2026-10-26, PAX-1047 web interface refresh: after rebuilding the local web interface, ordinary reloads in IAB and Chrome at `localhost:8147` retained the earlier JavaScript while loading the new Wasm. Expected keyboard-bridge changes to become active on refresh; observed Escape still ignored and temporary key logging absent. The served file contained the new handler (verified with curl); loading `127.0.0.1:8147` immediately delivered Escape and closed the editor. Workaround for verification: use a fresh local origin; investigate development bundle cache headers/invalidation. Diagnostic logging removed.

- 2026-10-26 — PAX-1039 website validation: after scrolling the release homepage down roughly 4800–5600 px, resizing from 1280×800 to 390×844 reports `reactive drain hit 100000 evaluations in one tick; deferring remaining work`. Expected a bounded responsive-layout update. Reproduces on both forced WebGPU and Vello CPU; no browser exception was observed. Evidence: `/private/tmp/pax-1039-migration/scroll-check.json` and `final-extended.json`; release Wasm SHA-256 `385371f94f9b03aa4941b405048cd257bb1ab00111e40fc0b1ec8dbd10edb223`. The warning is associated with resize, not the earlier scroll steps. Cause and relation to website layout work remain unestablished.

- 2026-10-26 — PAX-1039 debug smoke of `examples/fixtures/responsive-helpers`: renderer-global text resolves correctly for CPU and GPU, but the fixture's external Inter font remains `document.fonts.status === "loading"`; native text stays hidden and Playwright's font-ready screenshot wait times out after 30 s. Expected a visible helper panel. Both backends reproduce on the static debug server at port 18745. Value checks used attached DOM text instead; the main website uses bundled fonts and its screenshots pass. External font/network behavior is not yet diagnosed.
## 2026-10-16 — Pax formatter emits trailing spaces on wrapped attributes

While adding the website's shared-definition section, expected `pax-cli fmt`
output to pass `git diff --check`. Formatting `pax-website/src/authoring_section.pax`
and `shared_definition_demo.pax` leaves a space before newlines when attributes
wrap (for example after `width=100%` before a following `text=`). The same happens
in the existing `home_page.pax`. Reproduce with `target/debug/pax-cli fmt <file>`
then `git diff --check`; trim trailing spaces after formatting as a workaround.
No formatter behavior was changed in this content-only task.

Reconfirmed 2026-10-29 on `home_page.pax` and `feature_gallery/gallery.pax`:
wrapped attributes still fail `git diff --check` after formatting; trimming
trailing whitespace restores a clean diff.

## 2026-10-16 — Component data named `anchor` collides with layout shorthand

While prototyping the website's docking logo, a `Property<LogoAnchor>` named
`anchor` compiled and propagated, but also produced runtime warnings trying to
coerce the object into `anchor_x` and `anchor_y` Sizes. Expected the custom
component property to either be unambiguous or receive a compile-time diagnostic.
Reproduction: pass `anchor={self.logo_anchor}` to a component with that property;
`pax-cli dev logs --path pax-website` reports `Failed to resolve multi-axis
property`. Renamed the website property to `home_bounds`; no compiler/runtime
change was made.

## 2026-10-16 — Fixed-overlay logo cannot track native scrolling smoothly

The website prototype kept the hero logo in a fixed overlay, using measured
bounds and sampled `Scroller.scroll_pos_y` to translate/scale it into the nav.
Endpoint and interaction checks passed, but Zack observed discrete frame drops
and position leaps during the transition in the web preview. This approach
does not guarantee compositor-synchronized motion. Removed the moving-overlay
prototype: the hero logo now stays in the native scroller, and a separate,
finished nav logo only fades at visibility thresholds. No claim of a general
scroll-linked-transform fix or measured root-cause profile is made.

## 2026-10-20 — Reused preview origin rendered text without GPU artwork after rebase

After rebasing the website onto `a2e2189f7`, a fresh debug web build served at
`http://127.0.0.1:56470/` showed native text but left canvas backing sizes at
300×150. Ordinary browser reload and `?pax_mount_yield=0` did not recover the
artwork; the homepage also logged five reactive-drain budget warnings. Expected
the rebuilt preview to render normally. An isolated upstream website snapshot
with the same dependency lockfile rendered correctly on port 56471, and the
unchanged rebased build rendered correctly at `http://localhost:56470/`,
including the hero, nav, source drawer, and Paxflix route. This points to stale
origin-scoped browser state/assets, not a confirmed engine or merge defect.
Workaround: use a fresh preview origin after substantial runtime changes.
Browser screenshots and CLI captures were taken during this chat; CLI captures
are under `/tmp/pax-rebase-20261020/`. No renderer workaround was introduced.

## 2026-10-21 — Yielded startup replaced an authored scroller position with zero

Opening the website's `/calculator/` route with yielded mounting showed an empty
graph at the far edge of its plane; `?pax_mount_yield=0` centered it correctly.
Expected the same authored starting position in both modes. A browser frame
between `ScrollerCreate` and the first full `ScrollerUpdate` reported the DOM's
temporary zero offsets back into bound application state. The web host now waits
for the initial layout/position update before sampling that scroller, including
on the iOS-specific path. `scroller-initial-position.test.mjs` covers the split
creation/update sequence; no calculator-specific startup override is needed.

## 2026-10-22 — Escaped newlines remain literal in template string attributes

While styling the website's hero terminal, `Text text="first\nsecond"` rendered
the literal backslash and `n` instead of a line break. The string grammar accepts
escapes, but `PaxDeserializer` passes the inner source text through unchanged.
Actual newlines inside the quoted attribute produce the intended selectable
multiline text. Used that authoring form here; no language behavior changed.

## 2026-10-27 — Website iteration exhausted disk through accumulated compiler caches

An incremental `pax-cli run --path pax-website --target web --libdev` failed
writing `full.rmeta` with `No space left on device`, rather than producing the
expected preview. This worktree's website target directory occupied about 27 GB,
including 9.7 GB in the WebAssembly debug incremental cache across several build
configurations. With no website build running, removed only that regenerable
`pax-website/target/wasm32-unknown-unknown/debug/incremental` directory and retried;
source, dependencies' compiled outputs, and other worktrees were left intact.

## 2026-10-27 — Inline web button baseline shifts compact controls

Building a 52×28px theme pill with a 1px transparent native label placed the
button at y=39.5 although its native leaf transform requested y=27. The default
inline-block baseline added 12.5px, and Pax's authored clipping cut off the
displaced track. Setting `.button-styles` to `display: block` keeps the native
button in its authored rectangle; label alignment remains a TextStyle concern.

## 2026-10-27 — Section-local source drawers are not viewport overlays

At 390×844, opening the hero source drawer removed its measured preview and
left a 580px-tall viewer with an empty area below it. `height=100%` referred to
the section, not the viewport; Breakout does not escape a Scroller. Extracted
`ExampleSourceViewer` and mounted the mobile presentation above the page
Scroller, with explicit open/selection/theme bindings and a native backdrop.

## 2026-10-27 — Reactive-drain warning during source-viewer viewport resize

During web QA, resizing the open source viewer from 390×844 to 1280×900
preserved theme/selection and rendered the desktop split correctly, but dev
logs reported one `reactive drain hit 100000 evaluations in one tick; deferring
remaining work` warning, following two tile-window-escape resize warnings.
Mouse/keyboard controls remained responsive. The responsible reactive chain
has not been isolated; no runtime workaround was added for this observation.

## 2026-10-28 — Enter timelines can precede initial native text presentation

Opening the mobile source sheet after a page reload at 390×844 shows a brief
intermediate frame with the sliding surface and controls, before its title,
tabs, and code text appear. Expected a fully composed sheet throughout entry.
The web native pool creates Text leaves hidden and releases them through
`InitialTextPresentation` / `finishTextFrame`, independently of the component's
350ms `@in` clock. Captures in `/tmp/pax-sheet-open-02.png` and `03.png` show the
handoff. Moving the opaque backdrop with the sheet removes the previous
full-page blank interval; highlighting only the selected source avoids eager
work for hidden tabs. No arbitrary delay or bypass of text readiness was added.
Coordinating lifecycle entry with native presentation readiness remains open.

- 2026-10-29 — PAX-1056 web fixture validation: typed `Edited 🚀` into
  `Text editable=true text=bind:note` through browser key events. Expected normal
  caret advancement; observed reversed character order (`🚀 detidE`), while the
  bound echo correctly followed each update. Pasting the full string works.
  Repro: `examples/fixtures/text-bodies`, browser Text editing; investigate the
  existing content replacement/selection handling separately from body parsing.

- 2026-10-29 — Running `swift test --package-path
  pax-host/native/common/pax-swift-common --filter NativeTextTests` concurrently
  with Cargo tests created `.build` under a directory embedded by
  `pax-host/src/sources.rs`. Expected independent checks; Rust `include_dir!`
  raced Swift module-cache lock files and failed to read a removed file. Use
  `swift test --scratch-path /private/tmp/<task-build> ...` and keep generated
  Swift output out of the embedded source tree.

- 2026-10-29 — PAX-1056 native fixture launch: `target/debug/text-bodies`
  compiled successfully, but this session reported `Pax display clock unavailable
  (-6661); retrying on display/session wake`. Native AppKit text tests pass; an
  interactive native visual check remains unverified on this host.

- 2026-10-29 — Broad runtime validation during PAX-1056: two
  `viewport_proximity` mask tests fail identically on the working change and an
  untouched `git archive HEAD` baseline (`de7e1c095`), including serial execution.
  `masks_with_scroll_islands_disabled_use_presented_geometry` expects a nonzero
  mask at line 1255 but receives 0; `unclipped_scroll_overflow_updates_masks_outside_its_viewport`
  expects 0 at line 1288 but receives 5588616461809109485. Reproduce with
  `cargo test -p pax-runtime --test viewport_proximity -- --test-threads=1`.
  Remaining tests in that binary: 20 passed, 1 ignored.
