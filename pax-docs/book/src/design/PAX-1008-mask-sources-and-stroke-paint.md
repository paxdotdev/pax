# PAX-1008 — Component mask sources and stroke paint

Design and implementation checkpoint, September 27, 2026. Gradient paint and
shared capture APIs below are proposals, not shipped APIs. Zack approved normal
live component-tree lifecycle for mask sources, while excluding native source
capture. Implementation worktree: `codex/pax-1008-mask-strokes`, starting at
website checkpoint `0b5c68be1`.

## Findings and the missing initialization

`MaskInstance::handle_mount` allocates its second child with
`create_children_detached`, binds layout through `attach_sidecar_children`, and
runs control-flow expansion. Allocation already creates the source component's
properties. It does **not** run `ExpandedNode::recurse_mount`, however:

1. The component's user `@mount` does not run.
2. `ComponentInstance::handle_mount` does not build its internal template.
3. Handwriter's `_elements` remains empty and its internal Path does not exist.

There is consequently nothing for either an alpha-paint collector or an
offscreen rasterizer to draw. Group opacity capture does not itself solve this
initialization gap: it captures draws from an already-initialized scene.

The regression
`component_mask_source_initializes_handwriter_and_expands_its_template` uses
real Handwriter properties and its real mount handler, with a simple template
leaf. Before the fix its visible control succeeded and the mask source failed
with empty generated geometry. It now passes alongside native-suppression,
replacement/cleanup, primitive-mask and keyed-repeat invalidation tests:

```sh
cargo test -p pax-std --lib \
  component_mask_source_initializes_handwriter_and_expands_its_template \
  --offline --target-dir target -- --nocapture
```

That test is a runtime unit reproduction. The companion
`examples/src/mask-strokes` fixture now verifies root-level WGPU web rendering:
CMY content through a live Handwriter and a direct Path alpha source, at desktop
and 390px width. Debug pause/resume, removal/remount, and source-template hot
reload were exercised without duplicate native text. The release fixture also
executes the baked cartridge. Apple-native and Piet alpha rendering are untested.

The initial Scroller failure was separate from source initialization: restoring
the old initialization still reproduced the direct Path failure with nonempty
source geometry. The content primitive localized against the nearest
different-layer Frame/Mask, while alpha coverage localized against the actual
Scroller owner. Translated masks therefore separated paint from coverage.
Canvas primitives and lighting now share an owner-based transform calculation.
The regression failed before the fix and covers intermediate structural frames,
nested transformed owners, and root surfaces. The canonical fixture now keeps
both sources inside a Scroller and adds an offset Frame/Mask in a nested Scroller.
Both debug and baked release web builds render those masks at desktop and narrow
widths, including pause/resume and removal/remount. Independent nested and outer
scrolling was verified in release. All 169 runtime and 65 standard-library tests
pass, as do API generation and the book build. The website itself still needs
integration acceptance.

Calling ordinary mount on the sidecar without further work is unsafe. Mount
registers the node for lifecycle dispatch and native primitives allocate their
surfaces. Handwriter's invisible selectable Text would thus be mounted too.
Group can allocate a LiquidGlass surface, so suppressing only nodes whose layer
is `Native` is insufficient. Current Mask teardown also does not recursively
unmount its sidecar; a real source lifetime needs balanced teardown.

## Three distinct lanes

1. **Logical source lifetime:** create properties, run component initialization,
   expand components/control flow/slots, maintain reactive subscriptions and
   clocks, and dispose the resulting subtree.
2. **Subtree composition:** turn supported initialized GPU content into coverage
   or alpha, with correct transforms, clips, internal opacity, caching and
   ordering. This is where group opacity is relevant.
3. **Stroke paint:** accept a Fill-like paint at the public Stroke API and carry
   it into existing renderer paint buffers. This does not require a mask or a
   new subtree capture pass.

Zack explicitly excludes native Text and controls as sources unless/until those
have a GPU rendering implementation. Do not capture DOM/UIKit/AppKit content or
create replacement native controls. Keep meaningful accessible text outside the
mask. This scope restriction does not remove the lifecycle decision in lane 1.

## Recommended logical source contract

Introduce one internal, inherited **render-source participation mode**, distinct
from ordinary scene participation and from lexical/provider scope. Sources are
real, separately owned component instances, not borrowed visible nodes.

- Component `@mount` and logical template setup run once; ordinary reactive
  properties, named timelines and relevant tick/pre-render logic remain live.
- Component/control-flow/slot/Path initialization uses the usual mechanisms.
  Do not create a second partial implementation of component expansion.
- Native leaves do not create surfaces, native patch subscriptions, accessibility
  nodes or input targets. Unsupported leaves contribute no source paint. A
  debug diagnostic can distinguish unsupported sources from an empty Path.
- Split structural/logical setup from presentation work for mixed primitives
  such as Group and Frame. Do not globally discard native messages: that leaves
  native-resource bookkeeping/subscriptions running invisibly.
- Source nodes remain inspectable and reloadable through runtime identity, but
  never enter ordinary hit testing, focus, proximity events, visible canvas
  replay or native occlusion. Source paint invalidation targets its owning mask.
- The mode propagates to nested components, newly admitted keyed items,
  conditionals and projected children. A node cannot simultaneously belong to
  a presented scene and a source tree; share Properties/data, not node instances.
- Removal/remount and template/logic replacement run matching cleanup, release
  subscriptions and retained GPU resources, and prevent stale scheduled work.
  A hidden mask source is not an unmounted component.

User mount handlers may run application side effects just as they do in visible
instances. That implication should be explicit in the docs; it is not safe to
promise a pure raster-only operation for arbitrary Rust components. Public
input or viewport events are not fabricated for a source. This new lifetime
policy was explicitly approved by Zack on September 27.

PAX-1007 owns store/provider isolation: preserve its distinction from lexical
scope and avoid implementing a competing provider graph. PAX-1002 owns viewport
events; render-only source nodes should not masquerade as visible observers.

## Alpha collection versus retained subtree capture

| Choice | Advantages | Constraints |
| --- | --- | --- |
| A. Initialize sources, keep `resolve_alpha_mask_paints` | Narrowest Handwriter fix; preserves current primitive mask implementation | Separate paint/coverage extraction remains; source-side Frame/Mask clips stay unsupported; flattening inherited opacity differs from composed group opacity |
| B. Initialize sources, capture supported GPU draws | Reuses actual render semantics: source clips, draw ranges and internal opacity; less parallel shape-rendering logic | Requires explicit source render ownership and reusable retained capture; larger change and needs PAX-1004 base integration |

**Prefer B as the durable direction** if component mask sources should look like
their normally rendered GPU content. The current bounded implementation uses A
to prove the approved live-source lifetime independently of base integration,
retaining those stated limitations.
Neither option introduces native-source capture. Geometric masks can retain
their coverage collection; an alpha texture does not substitute for a native
geometric clip path.

PAX-1004 is completed at `a9b7aa7b0` (read using `git show`; its dedicated
worktree has been removed). It provides:

- Ordered retained GPU draw runs and nested opacity scopes.
- Tile-local scratch attachments and cropped retained texture storage.
- Content signatures covering geometry/paint/images/transforms/clips/lighting,
  excluding only the boundary's presentation opacity.
- Resource retirement and tested premultiplied texture composition.

The reusable seam is a **retained subtree surface** with an explicit coordinate
domain and content signature. Group opacity samples its premultiplied RGBA;
an alpha mask samples its alpha and optionally feathers it. Share allocation,
capture nesting, bounds/DPR, invalidation, and retirement rather than shoehorning
a mask into an opacity value. Keep geometric coverage separately available.

This is not a drop-in call to `begin_opacity_group`: current capture operates on
visible retained nodes in a canvas/tile, whereas source draws must not enter the
visible sequence. A source needs its own retained draw ownership and signature.
Mask feather needs an expanded capture gutter beyond content bounds to avoid
clipping soft edges. Nested capture must not sample the target being written.
Ancestor/content masks must not accidentally mask their own source capture.

Premultiplied group-texture alpha can be reused directly; do not multiply source
alpha by its color or apply ancestor opacity twice. Two overlapping opaque
source shapes under a 50% group should produce 50% alpha, not the 75% obtained
by separately fading them and source-over combining their alpha.

PAX-1004 still composes only the GPU portion of mixed content per canvas;
native text and separate Scroller surfaces fade independently. Piet uses
temporary canvases but does not retain group pixels. Do not advertise arbitrary
mixed-native capture, native alpha masks, or cross-surface capture as supported.

PAX-1004 is on main (verified at `a9b7aa7b0` on September 27). This worktree's
base still predates it. Implementing B should start after Zack directs
base integration. Do not copy an entire renderer diff or independently recreate
the landed group-opacity feature in this worktree.

## Gradient stroke proposal

The low-level `pax_gpu::Stroke` already has `fill`. Retained stroke geometry is
keyed separately from paint, and material/paint updates have an existing dirty
buffer path. High-level GPU adapters currently force `Fill::Solid(stroke.color)`;
Piet has a reusable `fill_to_piet_brush`. No new general paint shader is implied.

Recommended authoring addition, subject to review:

```pax
stroke={
    paint: @gradient {
        linear: { start: [0%, 0%] end: [100%, 0%] }
        0%: rgb(0, 220, 235)
        50%: rgb(245, 0, 180)
        100%: rgb(250, 225, 0)
    }
    width: 5px
    cap: StrokeCap::Round
}
```

- Add optional `paint: Property<Option<Fill>>`; absent means `Solid(color)`.
  Explicit paint wins, including transparent paint. Color remains a compatible
  shorthand/fallback and must not accidentally tint or multiply the gradient.
- Preserve existing Pax `stroke={color: ..., width: ...}` and color coercion.
  Adding a Rust struct field requires `..Default::default()` or a new explicit
  field in exhaustive Rust literals; call that migration out honestly.
- Prefer `paint` over overloading Color into a different type. `fill` would align
  with the low-level name but can be confused with the shape's interior fill.
- Resolve all paint coordinates against the complete local centerline bounds,
  before draw-range trimming; smooth geometry consistently. On a zero-extent
  axis expand symmetrically by stroke width (with a finite minimum). Open paths
  therefore have a stable domain. A spatial gradient is not an arclength rainbow.
- Keep percent coordinates attached to local geometry through transforms. Paint
  animation must not regenerate Handwriter glyphs or reset its writing range.

Required parity work goes beyond swapping the solid adapter:

1. Coercion, ToPaxValue, serde defaulting, equality/hash/interpolation, static
   type descriptors and runtime structured-property adapters. Fix the observed
   missing cap/join descriptor entries alongside the new field. Audit rich and
   baked cartridge construction with actual debug/release execution.
2. Every Stroke consumer: Path, Handwriter, Line, Rectangle and Ellipse; visible
   paint, coverage/occlusion alpha estimates, and alpha-mask extraction.
3. Piet currently trims the path and only then derives its brush bounds. Use a
   fixed brush resolved against the complete path instead. Alpha extraction
   currently converts stroke to an outline and derives gradient bounds from that
   outline; preserve the original paint domain independently of coverage.
4. Existing radial Fill semantics differ between visible GPU, alpha GPU and
   Piet (documented in Drawing). Reusing those three conversions blindly would
   introduce mismatched radial strokes. Settle one stroke contract and tests;
   do not silently change existing Fill radius semantics. Linear paint can ship
   separately if radial consistency requires a broader migration decision.
5. Measure geometry cache/tessellation and GPU resource churn while changing only
   paint stops. The visible retained stroke path separates these concerns, but
   current alpha collection tessellates outlined paths before its texture-cache
   check. Do not promise mask-geometry reuse until measured or improved.

## Acceptance sequence

1. Source lifetime is approved. Integrate the landed base under Zack's direction
   before implementing shared capture B. Confirm optional paint syntax and radial
   scope. No website composition changes in this task.
2. Implement lifecycle tests for mount/template generation, reactive updates,
   nested components/slots/if/keyed repeats, remount/reload and full disposal.
   Assert zero native create/update/delete patches or input/visibility
   registrations from unsupported source leaves.
3. Implement source composition and stroke paint in independently reviewable
   steps. Preserve geometric mask behavior unless separately approved; today a
   nonempty Path draw range contributes its complete geometric stroke outline,
   whereas alpha masks use the trimmed stroke.
4. Verify pixels for CMY-through-Handwriter, gradients directly on Handwriter,
   caps/joins, zero-area/open paths, transforms, partial alpha, nested clips and
   group opacity. Exercise pause/resume, resize, removal, hot reload and cached
   stationary states. Compare desktop/mobile web debug and baked release.
5. Run focused runtime/API/descriptor/baking tests; inspect GPU churn counters;
   test Piet where available and state native target limits. Update canonical
   Drawing and Compositing articles, public comments/generated references,
   example sources, bundled sources and pain points. Build the book and finish
   with formatting and `git diff --check`.

Return a small reusable fixture and website integration recipe, not an automatic
merge or a claim that the current website already has CMY handwriting.
