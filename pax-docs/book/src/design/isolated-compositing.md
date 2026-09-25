# PAX-1004 — Isolated group compositing: design checkpoint

Status: implementation in progress, September 25, 2026. Zack approved ordinary
`opacity` as subtree composition, separate native/vector fades initially, and
no legacy mode. Prove web WGPU and iOS/iPadOS before assessing macOS/Piet rollout.
This is a maintainer design note, not a claim that every target is validated.

## Contract and baseline

Compose a card's GPU image, gradient and background internally, then apply its
outer opacity once. Live native text and controls may fade separately for the
initial implementation. Preserve ordering above nested native Scrollers and
document the remaining mixed-content approximation. Keep input,
focus, accessibility, clipping, nested groups and the full exit lifetime.
PAX-1002 is not a prerequisite. PAX-915's filters/blend modes are separate.

PAX-1001's September 24 coordination comment keeps gradient crossfading inside
a shape's paint separate from this boundary. Neither task depends on the other.
Preserve the existing straight-alpha `geometry.wgsl` output and
`render_backend/mod.rs` `ALPHA_BLENDING` contract unless a coordinated change
is necessary; the alpha-mask path already has a different premultiplied
contract. Make any conversion for an offscreen intermediate explicit, including
zero-alpha handling and color space. Paint interpolation does not establish
the backdrop semantics required by future blend modes. The PAX-1001 worktree
at `/Users/zack/.codex/worktrees/500e/pax` is read-only reference.

The investigation starts at `090348859`. The read-only Paxflix reference is
`/Users/zack/.codex/worktrees/42d6/pax`, HEAD `ed5c2579b` (the previously inspected fixes are now committed).
Neither Paxflix nor its renderer fixes are present at this baseline. Do not
copy that checkout's whole diff or redeploy its running devices incidentally.

The ticket and the reference README record two separate failures: descendant
opacity changes internal appearance, and physical Argus frame callbacks stall.
Two eight-path masks at 1320×2478 and 1320×2868 repeatedly cost approximately
18.5ms each to rasterize. Those CPU/display-link observations are not GPU
timestamps or displayed-frame counts. Render-call stalls need independent
attribution even after native mask work is reduced.

## Code-backed architecture map

| Seam | First implementation | Remaining work |
| --- | --- | --- |
| Opacity inheritance | `computed_opacity_scopes` preserves each ancestor factor; `computed_opacity` remains available for native presentation | Avoid unnecessary CPU descendant replay during fades |
| Canvas coordinates | Existing tile-local transforms and clip coordinates are reused | Translation still repaints cached content |
| Ordering | Stable scope IDs partition retained nodes into ordered runs within each canvas | Exact composition across native/Scroller surfaces |
| Native masks | Existing geometric coverage and estimated opacity remain | Animated mask costs and approximation remain measurable limitations |
| Web hosts | Existing live DOM and Scroller hosts; WGPU and Piet subtree composition | Possible exact platform grouping later |
| Apple hosts | Existing native identity with synchronous iOS/macOS native/mask/Metal publication; focused Molino and macOS acceptance | Broader accessibility and catalog-scale physical-device profiling |
| GPU allocations | Visible-bounds cached group textures; reusable tile-sized scratch attachments | Profile memory at realistic nesting and catalog size |
| Invalidation | Content signatures exclude outer opacity, include paints/images/transforms/clips/lighting; rescue, final removal and shared-image refresh tested | Network decoding and broader application acceptance |

An ordinary Group does not currently materialize a native container, except
for its LiquidGlass surface. Adding native Group opacity alone cannot capture
the GPU canvas beneath it. A GPU-only texture likewise cannot capture live
native text, focus or accessibility.

## Approved default and implementation

Ordinary `opacity` establishes an authored subtree scope. There is no new
`compositing` setting and no legacy authoring mode. The runtime keeps the
individual ancestor factors, including zero and one, separately from the world
opacity used by native presentation and occlusion. Nested authored scopes
compose recursively; descendant paint alpha remains part of the content.

A subtree does not already own a canvas: vector descendants share root or
Scroller canvas tiles. Each retained canvas node therefore carries stable
opacity ancestry. The renderer constructs ordered runs from those scopes;
escaping content cannot pull unrelated draws across its render position.
Scopes crossing physical native/Scroller surfaces currently fade independently,
as part of the approved heterogeneous-content approximation.

The GPU path renders a scope into transparent scratch attachments using the
existing tile coordinates, clips and lighting. It retains only the visible
content bounds in a sampled texture. Scratch attachments are reused across
sibling groups. Nested scopes need simultaneous scratch attachments, so maximum
active nesting still affects memory. These costs need profiling on Argus.

Geometry and image shaders keep their existing straight-alpha output and
source-over blending. The resulting group texture contains premultiplied color
in the backend's blending color space. Its separate composition shader scales
all four components by the boundary opacity and uses premultiplied source-over.
The existing paint/gradient shader contract remains unchanged for PAX-1001.

Content signatures include local geometry, paints, images, transforms, clip
geometry/alpha, lighting, resolution and descendant scopes. A group's own alpha
is excluded: opacity-only changes sample cached content without replaying its
local GPU draws. Runtime descendants still receive opacity invalidations in
this first implementation; avoiding that CPU traversal is a separate optimization.
Outer translation currently invalidates content positions and native masks.
No reduction in Argus's lower-Scroller CPU mask work is claimed yet.

Ordinary Group overflow and unclippable ordering remain intact. Opacity alone
does not unmount controls, alter event policy or change the existing exit
retirement lifetime. Focused native identity and input checks pass; broader
accessibility coverage remains pending. No screenshots, custom glyph renderer or form-control
replacement are introduced.

## Platform strategy and remaining proof

The first proof uses internal WGPU scopes, leaving existing native hosts live.
This reduces topology changes and permits the approved approximation. It does
not provide exact native/vector group blending, eliminate approximate native
coverage masks, or compose vector content across separate native Scroller
surfaces into a single texture.

A future shared platform host could supply exact blending for suitable ordering:
browser stacking contexts or Core Animation group opacity can retain native
controls and accessibility. General interleaving still requires ordered canvas
runs and explicit ownership. See [W3C compositing](https://www.w3.org/TR/compositing-1/),
[Apple group opacity](https://developer.apple.com/documentation/quartzcore/calayer/allowsgroupopacity),
and [Metal transaction presentation](https://developer.apple.com/documentation/quartzcore/cametallayer/presentswithtransaction).
This remains an optimization/design option rather than a prerequisite.

The iOS implementation includes synchronous native scene/mask publication,
immediate text-layer updates and Metal transaction presentation from the
Paxflix reference, plus viewport-bounded backing allocation for the catalog.
Do not restore asynchronous stale masks to improve reported frame time.
The macOS host now also batches tick, native updates and rendering in a disabled-
actions Core Animation transaction and uses Metal transaction presentation.
Its focused fixture preserves native input through an interrupted exit. The
separate fixture was deployed to Molino with Zack's authorization; Paxflix's
app and Argus were left untouched.

The backend capability is internal, not a builder-selectable legacy mode.
WGPU was implemented first; browser Piet now implements the same subtree-opacity
contract through ordered canvas composition.
Public documentation must distinguish validated
behavior from pending targets.

## Alternatives raised by Zack: compensated masks and separate fades

These alternatives remain worth evaluating against implementation cost. They
have different guarantees from an isolated group. The following derivations
use ordinary source-over in one consistent compositing color space, with
unassociated source colors and premultiplied output/backdrop. They are derived
from the [source-over equations](https://www.w3.org/TR/compositing-1/#simplealphacompositing),
not from an assumption that masks can only have uniform alpha.

### Reordering one native/vector pair

Let `S` be a GPU source with alpha `a`, and `N` a native surface with per-pixel
alpha `b`. We want `S over N over backdrop`, but physically present `N over S`.
Applying native mask multiplier `m = 1-a` gives the correct native contribution
`b(1-a)N`. With GPU alpha left at `a`, however, the GPU contribution becomes
`a[1-b(1-a)]S`, rather than the desired `aS`. A native mask alone cannot match
both coefficients for arbitrary colors and intermediate alpha.

Changing GPU alpha as well **can** make this pair exact:

```text
native mask multiplier = 1-a
GPU alpha              = a / [1-b(1-a)]
```

For opaque native pixels (`b=1`), the GPU source stays opaque while the native
surface fades away. For native holes (`b=0`), GPU alpha must instead be `a`.
Antialiased glyph edges require the intermediate values of `b`. Transparent
content is not a mathematical impossibility here; it makes the required
per-pixel information essential. At the degenerate `a=0,b=1` endpoint the GPU
result is fully hidden and its alpha may be defined as zero.

### Fading a composed native/vector pair

For native content `N` above GPU content `S` *within* a group fading by `t`,
independent fades give GPU contribution `ta(1-tb)S`. Group opacity requires
`ta(1-b)S`. Exact compensation for this two-layer case is:

```text
native alpha = tb
GPU alpha    = ta(1-b) / (1-tb)
```

The denominator-zero endpoint `t=b=1` is fully covered by native content;
the hidden GPU alpha can be chosen without affecting output. This is different
from compensating for the reversed physical layer order above.

A concrete separate-fade failure is an opaque white native glyph over an
opaque black GPU card, against a white backdrop. A 50% group fade leaves the
glyph pixel white. Fading the two layers independently makes that pixel 75%
white because the fading glyph exposes the independently fading black card.
Internally compositing all vector content first would stabilize the
image/gradient relationship, but would not fix this native/vector overlap.

Both compensation formulas were checked against direct source-over over
10,000 seeded randomized premultiplied RGBA cases, including translucent
backdrops, with worst component error below `5e-16`. This validates the
two-layer algebra only, not a Pax implementation or arbitrary interleaving.

### Engineering assessment

The current native mask protocol carries vector paths and estimated opacity,
not native glyph/control pixel alpha. The compensated solution therefore needs
new native-alpha capture/coverage data, matching rasterization and color-space
contracts, GPU alpha adjustment, and extension to multiple interleaved layers.
Known endpoint opacities do not provide that missing coverage. Changing the
existing full-viewport CPU mask every frame also leaves the measured Argus
cost unresolved; caching native alpha could help static content but must track
native content invalidation correctly.

Separate native and internally composed vector fades are the approved initial
quality tradeoff. They stabilize image/gradient/background relationships while
retaining the described native/vector overlap limitation. Exact native pixel
coverage is not required for this first implementation.

## Implementation validation

- The existing Image opacity fix is ported with its runtime regression.
- Metal pixel tests establish uniform overlap alpha at 0, .25, .5, .75 and 1,
  nested scopes and translucent image/vector composition, clipping, restoration
  from zero, and the screenshot mirror path.
- Those tests also establish that outer-opacity changes do not replay cached
  GPU content, rebuild vector buffers, recreate image draw buffers or upload
  source textures. Paint, clip and node-removal changes invalidate content.
  A shared image updated outside the group also invalidates a cached grouped
  consumer without requiring that consumer to replay its draw declaration.
- Web debug and baked release overlap probes match. iPhone and iPad simulator
  probes match too; native input and its caret survive opacity changes and
  movement, including zero followed by restoration. iPhone rotation preserves
  the probe appearance. The expanded fixture checks interrupted exit rescue,
  completed removal/remount, delayed decoded artwork and changed paint on web.
  Runtime tests preserve scope/draw IDs through rescue and retire them at final
  removal. macOS and Molino preserve edited native input during reversal, and
  Zack confirmed the physical iPad's overlap probes match.
- Checkpoint validation passed: 253 Rust tests across runtime, standard primitives and
  GPU crates; seven Metal hardware tests; six native presentation/visibility
  Swift tests; two baking roundtrips; API documentation generation and mdBook.
  The checkpoint fixture built for debug/release web and the iOS simulator.
- The September 25 extension passes five clock/lifecycle runtime tests, eighteen
  GPU unit tests, all nine Metal hardware tests, six native presentation/visibility
  Swift tests and mdBook. No compiler/runtime schema changes were introduced.
  The corrected fixture builds and runs in debug and baked release web, and its
  signed iPadOS release build is installed on Molino for the seam recheck.

### Fractional crop regression

Zack reported a flickering bright band at the image/gradient seam on Molino
during a translating fade. ImageFit::Fill deliberately draws oversized artwork
through a rectangular crop. The scissor optimization rounded that crop outward
to physical pixels while the overlaid gradient ended at its actual fractional
position. A Metal regression reproduced the exposed strip at a 1/64-pixel
translation. Rectangular clips now use the scissor fast path only when all four
edges align with physical pixels; fractional edges use stencil sample coverage.
The regression sweeps 64 vertical offsets at DPR 1, 1.25, 2 and 3, with group
alpha .25, .5, .75 and 1. Zack confirmed the corrected release build removes
the seam on Molino.

### Bounded Molino profiling

The release fixture can run twelve alternating exit/entrance phases after a
three-second settling period. Opt-in CPU instrumentation reports display-link
frame phases and uncached native-mask raster duration; it does not measure GPU
execution or displayed frames. See the fixture README for the launch command.

On Molino (13-inch M4 iPad Pro), a repeated instrumented run before the crop fix completed with
seven mounts and six exits. During the scripted transitions it rasterized 961
four-path masks at 2672×1296 pixels, with median 3.559ms and maximum 12.458ms
per call. Across the 16 one-second transition windows, the maximum callback
interval was 77.19ms, maximum total CPU frame time 31.69ms, and maximum native
phase time 21.77ms. These windows contain idle time and diagnostic logging adds
overhead. The scene settled to approximately .31ms total CPU frame time with
no further mask raster calls after the script completed.

An earlier run continued rasterizing after completion; this did not recur in
either a fresh idle run or the repeated scripted run, so its cause is unresolved
and it is not evidence of a confirmed idle invalidation bug. These five-row
fixture measurements do not replace the larger Paxflix/Argus investigation or
establish a performance improvement against that baseline. Animated native
masks still consume material CPU time.

## Regression and documentation checklist

- Fixed alpha 0, .25, .5, .75, 1 against a compose-then-fade pixel reference.
  Two opaque overlapping shapes at .5 must have .5 group coverage throughout,
  versus .75 in their overlap under inherited descendant opacity.
- Native label/control above and below translucent GPU content; outside native
  and canvas siblings above/below the group; nested boundaries and nested clips.
- Card crossing row tiles/gutters, internal/external Scrollers, retained scroll
  positions, first frame, interrupted/reversed exit, final resource removal.
- Input/keyboard focus/accessibility, late artwork, theme changes, rotation,
  zero opacity followed by restoration, and stable control identity.
- Instrument no texture upload/local replay for pure opacity. Lower Scroller
  mask signatures still change with opacity in this implementation; reducing
  that native-mask dependency remains separate work.
- Audit `pax-manifest/{program_ir,binary,rust_manifest}`, compiler property
  descriptors/cartridge templates, common property tables if touched, message
  serialization and both chassis consumers. Test literal/bound opacity
  properties and nested groups through debug and baked execution.
- Canonical article: `compositing-effects.md` (boundary table, subtree opacity,
  native coverage, Scroller islands). Related guidance: `animation-motion.md`,
  `scrolling-viewports.md`, `accessibility-native-controls.md`, and Group API
  source comments. The canonical article describes WGPU and browser Piet
  subtree opacity, their different reuse strategies, and the mixed-content
  approximation.

## Baking audit

No manifest, common-property schema, message format or binary version changes
are introduced. Existing common `opacity` values are decoded by the normal
cartridge paths; runtime attachment derives opacity ancestry from them. The
binary-manifest and ProgramIR roundtrip tests pass. The baked web fixture also
builds and runs with the same composed-overlap result as the debug fixture.

## Backend assessment

Native macOS uses the same WGPU renderer, and the Metal pixel tests execute on
macOS. The native debug fixture also passes overlap and input/reversal checks
with synchronous frame publication. This is a focused acceptance pass, not a
full accessibility or release-app certification.

Browser Piet records each dirty canvas draw with its opacity ancestry and a
snapshot of its transform/clip state. At flush, contiguous scopes compose on
transparent Canvas2D surfaces before their boundary opacity is applied. Clip
paths retain the transform from clip installation; a later primitive transform
does not move them, and composition does not apply the clips a second time.
Unrelated draws retain their order even if they split a scope into multiple runs.

One scratch canvas per simultaneously active translucent nesting level is
reused across sibling runs and dirty frames, per physical tile. Opacity one
bypasses allocation and opacity zero skips painting. Tile retargeting or resizing
discards scratch surfaces so their coordinate spaces cannot become stale. Piet
still replays dirty content; this does not add WGPU-style retained group pixels.
The chassis supplies surface creation/composition through `PietSurface`, avoiding
CPU pixel readback or native-control snapshots.

The forced-Piet fixture passes debug and baked release browser checks for the
overlap reference, native input/reversal, zero/restoration, full exit/remount,
delayed artwork, changed paint, and scrolling under the card. Six focused Piet
tests cover ordered nesting, interleaved siblings, scratch reuse, endpoint work,
image paint alpha, clip-state restoration and existing tile replay priorities.
The runtime/standard-primitive suite passes 241 tests. These browser checks use
the local Chromium host; physical iOS WebKit remains a separate acceptance case.
No public legacy flag was introduced.

After integration with PAX-1001, paint mixtures remain single accumulated paints
inside their opacity groups, including Piet scratch surfaces. Eight focused Piet
tests pass, including the mixture/group regression and radial-geometry test.
The combined Rust suites pass 282 tests; 13 Metal pixel tests cover both gradient
mixtures and retained subtree composition, including cache invalidation when a
mixture changes. All 24 shared Swift tests and the regenerated documentation build
pass. Earlier browser acceptance above predates this integration.

## Catalog-scale native mask measurements

The September 25 controlled release run uses the Paxflix snapshot on main:
35 films, 160 cards, sixteen horizontal Scrollers and one vertical Scroller.
Molino (13-inch M4 iPad Pro) runs the same baked Rust cartridge in each comparison.
Only the generated native mask bitmap format changes between RGBA and A8.
After six seconds of settling, an automated sequence makes 24 alternating open/close steps (12 pairs) across three films
at one-second intervals, switching theme halfway through. A8 runs
bracket the RGBA run. Frame and mask instrumentation are enabled for all runs.

| Eight-path mask | RGBA median / p95 | A8 first median / p95 | A8 repeat median / p95 |
| --- | --- | --- | --- |
| 2752 × 1864 | 15.068 / 15.497 ms | 6.402 / 6.557 ms | 6.429 / 6.602 ms |
| 2752 × 2064 | 15.124 / 15.338 ms | 6.391 / 6.486 ms | 6.416 / 6.501 ms |

A8 reduces CPU rasterization time by about 57–58% for these masks. Their combined
bitmap storage falls from 41.24 MiB to 10.31 MiB, allowing both to fit in the
existing 24 MiB raster cache. Faster animation produces more raster samples
(186–187 per size versus 141); raw sample counts are not a regression measure.
These figures describe CPU mask bitmaps, not the compositor's GPU texture format.

Across the 24 one-second windows, the median of each window's maximum native
phase falls from 45.43 ms to 27.72 / 27.79 ms. Settled idle CPU frame work remains
about 4.7 ms, with no new rasterization. Whole-frame worst cases do **not** improve:
the maximum measured CPU frame is 121 ms for RGBA versus 156 / 151 ms for A8,
with cold artwork/rendering stalls still present. These are instrumented CPU
measurements, not displayed frame-rate or GPU timings. Further improvements need
to address remaining raster work and resource stalls independently.

The comparison predates the late-mount theme-initialization fix described in
the pain points; both formats ran the same initialization behavior. Current
Piet release catalog checks cover both themes, interrupted toggles, horizontal
shelf scrolling, and detail dismissal preserving the shelf's scroll position.

The late-mount regression raises the Rust total to 283 tests. A baked Molino
trace now reports solid black from the first detail frame, where the pre-fix
trace began at the Rectangle default and eased to black. Theme changes after
mounting still animate. No manifest schema or baking-format change is needed;
the fix is shared runtime mount ordering and is exercised by the release app.
Zack confirmed on Molino that the bright entrance phase is gone. The final
normal release is installed there, and the rebuilt Piet release passes dark/light
detail opening and dismissal with no browser errors.
