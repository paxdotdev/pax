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
- 2026-10-10 — The surface trace inspector's 400ms phone-scroll probe on
  `pax-website` lost its start/end events from the 2,048-event ring before the
  snapshot was opened. Expected a complete bounded probe; per-phase samples
  plus tile events exceeded retention. The retained tail cannot establish a
  gap-free whole probe. Used a separately captured native-wheel interval for
  ordering evidence instead; PAX-1014.
- 2026-10-09 — A web-interface-only TypeScript edit followed by the same
  `pax-cli build --target web --path pax-website --release --libdev` command
  started another `pax-website` Rust compilation despite unchanged Rust source.
  Expected only interface rebundling; the preceding release Cargo phase took
  461.58s. Canceled the duplicate compile, ran the interface's
  `build-interface.sh`, and copied its generated JS into the already-completed
  release output for verification. An interface-only build/reload path would
  avoid this feedback-loop cost. Observed on `518929f22` plus PAX-1014 changes.
- 2026-10-08 — Profiling `pax-website` after chassis edits: expected a local
  release build to include TypeScript changes, but root-level website builds
  do not auto-enable libdev (only `examples/src` and `tests/src` do). The build
  reused the old interface bundle while rebuilding Wasm. Use `--libdev` for
  website library-development builds, or explicitly run `build-interface.sh`.
  Also verify fresh Wasm loading: the in-app browser reused the prior asset on
  reload during this run; a fresh localhost port loaded it correctly.
  Evidence and profiling context: PAX-1014.
- 2026-10-08 — Formatting the Paxflix templates with `pax-cli format` adds
  trailing spaces before wrapped attributes, causing `git diff --check` to
  fail. Expected whitespace-clean formatted source; observed on `lib.pax`,
  `movie_card.pax`, and `movie_shelf.pax`. Removed trailing spaces after
  formatting; unrelated formatter-only changes were discarded.
  Reproduced Oct 10 on `pax-website/src/feature_gallery/marquee.pax` with
  `pax-cli fmt`: the wrapped `ScrollerHost` opening line gains a trailing space.
  Removed that space after formatting; `fmt --check` and whitespace-clean output
  cannot both pass until the formatter is corrected.
- 2026-10-11 — Profiling a 720px vertical wheel scroll in the website's embedded
  Paxflix phone: expected timely presentation after image submission; the tile's
  images submitted 6.5ms after visible demand, but `visible-unready` lasted 34.6ms.
  Another 10.4ms visible draw plus 11.6ms remaining draw ran synchronously afterward.
  All 37 unique images were already CPU-ready. Earlier submission alone does not
  establish earlier browser presentation; evaluate a safe host-yield boundary.
  Trace from the release preview on port 50515; findings and follow-up: PAX-1014.
- 2026-10-11 — Reversal testing on release port 50516: a warmed 540px phone
  reversal had no visible-unready transition but still included a 76.5ms CPU
  frame. A 1080px reversal beyond the prepared footprint exposed one shelf for
  36.2ms; new initialization was visible/incoming-first, not obsolete far-side
  work. Prepared pixels and input/frame responsiveness remain separate concerns.
  Concurrent builds make these diagnostic samples, not a controlled benchmark;
  reproduction and trace details are recorded in PAX-1014.
- 2026-10-12 — `cargo test -p pax-chassis-web --lib --features designtime`
  fails to compile the existing image-loading test fixture: it calls
  `PaxEngine::new_empty`, which is absent with `designtime` enabled
  (`src/image_loading/tests.rs:124`). Expected both configurations to test.
  Reproduced in the source at checkpoint `58b90765c`; use the default chassis
  tests, designtime chassis check, and both runtime test configurations pending
  a feature-aware fixture constructor. Encountered during PAX-1014 validation.
