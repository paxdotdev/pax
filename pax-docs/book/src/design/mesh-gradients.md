# PAX-1011 — Animatable mesh-gradient paint: proposal

Status: proposed, September 29, 2026. No public API or renderer implementation
is approved or present. This note is the design return value for Zack's review,
not public documentation of a supported capability.

Worktree: `/Users/zack/.codex/worktrees/mesh-gradients/pax`, created from
`226ea7a299706b7c31b867abb392a99f1eae03e5` on `zb/website`. The base includes
PAX-1008's paint layers and component mask sources. Uncommitted website styling
is deliberately excluded. PAX-1011 does not block PAX-869's temporary gradient.

## Recommendation

Add `Paint::MeshGradient(MeshGradient)`: a bounded rectangular grid of colored
anchors, connected by automatically derived bicubic patches. Animate anchor
positions and colors through ordinary Pax bindings. Start with a 3×3 CMY
fixture, supporting grids from 2×2 through 8×8 anchors in the first version.

Render the patch field into retained GPU paint storage, then sample it through
the existing shape/stroke paint path. This is a real mesh with a curved
coordinate mapping and interpolated color field, not layered radial blobs.
The initial supported implementation is WGPU; prove web debug and baked release
first. Native WGPU uses the same renderer but requires separate smoke tests.

| Model | Benefit | Cost / limitation | Recommendation |
| --- | --- | --- | --- |
| Colored triangle grid | Simple barycentric interpolation; inexpensive and naturally bounded | Color derivatives change at triangle edges; deformation can expose the chosen diagonals | Useful future faceted mode, not the smooth visual requested here |
| Bilinear quadrilateral grid | Few parameters and no arbitrary diagonal in the mathematical field | Derivative discontinuities between cells; warped-quad inversion or tessellation still needed | Not the default smooth model |
| Automatic bicubic anchor grid | Compact authoring, interpolated anchors, smooth shared boundaries, independent point/color animation | Overshoot and folds need explicit semantics; requires patch evaluation and paint storage | First version |
| Explicit Coons / tensor-product Bézier patches | Precise artist-controlled edges and interior handles | Much larger authoring surface; joining and color continuity become author responsibilities | Defer explicit handles and arbitrary patch topology |

The grid/control-point distinction is established prior art, not a proposed
compatibility promise: [Apple's mesh API](https://developer.apple.com/documentation/swiftui/meshgradient/init(width:height:bezierpoints:resolvedcolors:background:smoothscolors:colorspace:))
exposes a colored grid with optional explicit Bézier controls; [Cairo mesh
patterns](https://www.cairographics.org/manual-1.12.4/cairo-cairo-pattern-t.html#cairo-pattern-create-mesh)
use tensor-product patches and also describe Coons patches. Pax's automatic
tangent policy and authoring syntax below are its own proposal.

## Proposed authoring surface

Keep `@gradient`, adding a mutually exclusive `mesh` shape block. Use rows of
point/color records so dimension and color arrays cannot drift apart. This is
illustrative, not currently accepted syntax:

```pax
<Rectangle width=100% height=100%
    fill={
        paint: @gradient {
            mesh: {
                rows: [
                    [
                        {position: [0%, 0%], color: CYAN},
                        {position: [100%, 0%], color: FUCHSIA}
                    ],
                    [
                        {position: [0%, 100%], color: YELLOW},
                        {position: [100%, 100%], color: CYAN}
                    ]
                ]
            }
        }
        material: {Material::unlit()}
    }/>
```

The intended Rust value model is `MeshGradient { rows: Vec<Vec<MeshPoint>> }`,
where `MeshPoint` contains `position: (Size, Size)` and `color: Color`. These
are immutable paint values like today's gradient stops, not independently
mounted components or nested `Property` owners. Updating a bound position or
color rebuilds that paint value; it must not remount the primitive or its mask.
`rows: {self.mesh_rows}` is equally important as the literal form.

Rules:

- Rows are top-to-bottom in parameter space; entries are left-to-right. All
  rows have equal length. A mesh has no one-dimensional stop list.
- Each coordinate uses ordinary `Size`: percentages, logical pixels, and
  combined units. Percentages resolve against the stable full paint bounds,
  including their origin. Unitless normalized 0–1 coordinates are not a
  second coordinate convention. Authors use `50%`, not an ambiguous `0.5`.
- `rgba(...)` carries anchor alpha; no separate opacity field.
- `Fill`, `Stroke.paint`, layered paints, and explicit `Paint` values accept
  the variant. No new `Mesh` scene node or website-specific API.
- Animate points/colors to morph the mesh. Replacing one complete `Paint`
  with another through an automatic settings transition retains today's
  sampled-paint crossfade semantics, even for matching grids. Do not introduce
  surprising topology-dependent interpolation in `Paint::interpolate`.
- No repeat/spread, background, explicit handles, color-space selector, or
  arbitrary connectivity in the first API. Ordinary lower paint layers can
  provide a background.

## Patch, color, and overlap semantics

Use separable uniform cubic Hermite interpolation over the anchor lattice,
with centered finite-difference derivatives at interior anchors and one-sided
differences at the boundary. Derive mixed derivatives from the same shared
lattice. Equivalently this is an interpolating bicubic patch family whose
edge values and derivatives are shared, rather than independently authored
patches. The mathematical interior is C1 in parameter space before clipping
and color clamping; invertible geometry then gives smooth spatial color.
Tessellation is a rendering approximation with an explicit error budget.

Use the same patch basis for positions and colors. Interpolate color in
premultiplied, encoded-sRGB RGBA, matching the current numeric color channel
convention rather than introducing a global linear-light conversion. Clamp
the evaluated alpha to [0,1] and premultiplied RGB to [0,alpha], then safely
unpremultiply at the existing straight-alpha paint sampler boundary. This
prevents transparent colors contributing hidden hue. Cubic interpolation can
overshoot; clamping can flatten saturated areas, which the fixture must expose.
It is not a promise of perceptual or linear-light interpolation. Leave existing
linear/radial sampling unchanged. Verify surface-format differences separately.

Shared patch edges must use identical subdivision vertices and color samples:
no cracks, double-alpha seams, or independently antialiased internal edges.
Degenerate zero-area triangles contribute no coverage. Nonfinite geometry,
nonfinite colors, ragged/undersized/oversized grids produce a clear diagnostic
and no paint, never truncation or a substitute linear gradient. Statically
known malformed input should be rejected before mount; dynamic input must fail
safely with deduplicated diagnostics.

For finite folded/overlapping meshes, recommend deterministic **replacement**
within the paint field, not source-over accumulation: later row-major patches
win; within a patch, later row-major parameter triangles win. This prevents a
single translucent paint becoming more opaque where its own parameterization
overlaps. It deliberately makes folds a visible artistic choice rather than
an expensive global-injectivity validation problem. Exact fold boundaries are
tessellation-dependent; the proving animation keeps the grid nonfolding.
If Zack prefers folds to be invalid, decide that now: robust curved-patch fold
certification is additional work, not just checking the anchor quadrilaterals.

Outside the mesh's actual covered domain the paint is transparent. Anchors
may lie beyond the consumer bounds. Keep outer anchors covering the desired
paint area when moving interior points; use a lower fill layer for intentional
gaps. The mesh does not expand a primitive's geometry or its hit bounds.

## Renderer design and approval boundary

The existing `GpuGradient` record is 256 bytes with eight color stops; the
geometry shader samples solid/linear/radial paints and weighted mixtures.
There is no mesh sampler or general paint-texture atlas today. Do not pretend
this is only adding an enum arm or reuse the source-alpha texture binding.

Proposed bounded extension:

1. Resolve mesh points in the existing stable paint domain. Generate compact
   bicubic coefficients and shared patch topology. Keep this out of shape
   tessellation and source-component lifecycle.
2. Rasterize only visible required paint domains into retained, premultiplied
   GPU paint textures. Use GPU patch evaluation and reusable parameter-grid
   indices; vary subdivision density for geometry and color error, with a
   cap and hysteresis to avoid topology churn. Begin with conservative shared
   subdivision per connected grid; optimize per-patch density only if needed.
3. Add a small paint texture atlas/page abstraction and a mesh descriptor
   (domain mapping, page/region, generation). Mesh texels are paint resources,
   not screenshots, scene nodes, image assets, or group-compositing boundaries.
   A batch must bind the page containing all its mesh terms, including paint
   crossfade endpoints; partition batches without changing scene order. Atlas
   growth, full pages, and a blend exceeding page capacity need explicit tested
   handling, not silent missing terms or unbounded allocation.
4. Extend the shared paint sampler to sample that descriptor; apply ordinary
   material, alpha mask, opacity, and clipping afterward. Existing retained
   subtree capture then sees exactly the same mesh-colored draws.
5. Preserve paint fields during Path/Handwriter reveal: derive percentages from
   full smoothed source bounds, not the currently revealed segment or stroke
   width. A zero-area path uses the same canonical paint-bounds policy as
   existing gradients. Per-glyph direct Handwriter strokes retain their
   current per-path domain; a rectangle behind a Handwriter mask produces one
   continuous word-wide field.

Texture resolution follows projected physical size (DPR and affine scale),
with quality buckets/hysteresis. Translation and rotation should reuse paint
texels unless the required covered domain changes. Anisotropic scale may
increase the required resolution. Guard texels and a precise domain test must
prevent atlas bleed or edge-color clamping outside the mesh.

This paint-storage/sampler addition is the principal architecture approval
gate. First build an internal renderer spike comparing a 3×3 bicubic field
against a high-resolution CPU reference. If error-bounded subdivision,
crossfade resource binding, or atlas limits require a broad renderer rewrite,
stop and return that evidence. Do not silently reduce the mesh model.

## Retention and performance contract

- Separate topology, resolved positions, colors, paint domain, texture
  resolution, and presentation signatures. A color-only change must not
  recreate indices, retessellate a Path, regenerate Handwriter glyphs, remount
  nodes, or invalidate unrelated paints. It does redraw the affected paint
  texture and any cached composited result that contains it.
- Point motion uploads changed coefficients and redraws mesh paint; it may
  change mesh sampling density, but not consumer path geometry.
- Resource identity is stable per retained paint slot. A new content hash each
  frame must update/reuse that resource, not preserve every historical frame
  in an unbounded content-addressed cache. Share identical static data where
  correct; use copy-on-write when one consumer begins animating.
- Invisible meshes do no paint raster work. Bound textures/buffers to live
  references and existing retention policy; removal/remount releases resources.
  Shared use in ordinary content and mask capture must not generate duplicate
  paint textures for the same domain/quality.
- Record mesh coefficient upload bytes, texture allocations/bytes, paint
  raster passes, shape tessellation counts, cache hits/misses, and CPU/GPU timing
  where available. Reuse/extend `ResourceChurnStats`. No universal FPS promise.

Measure warm/cold and paused/animated 3×3 and 8×8 cases, color-only vs point
motion, one vs sixteen simultaneous consumers, 390px/mobile and desktop sizes,
DPR 1/2, unmasked vs Handwriter mask vs nested opacity, and scrolling/culling.
Report median/tail frame work, memory high-water marks, and known timestamp
limitations. Compare to solid and existing linear-gradient baselines.

## Compatibility, baking, tests, and documentation

Audit all touched boundaries, not just serde deriving the new enum:

- `pax-runtime-api/src/drawing.rs`: Paint/mesh values, equality/hash,
  alpha-factor/visibility estimates, coercion, ToPaxValue, interpolation and
  public API comments. Static type descriptors must match runtime structs.
- `pax-language`: grammar, formatter, literal/object roundtrip and diagnostics.
- `pax-manifest`: parsing/display, GradientShapeDefinition, rich JSON,
  program_ir sanitization, binary format/version policy and rust_manifest.
- `pax-compiler/src/lib.rs` and `pax-runtime/src/cartridge.rs`: dependency
  traversal and gradient evaluation must handle expressions deep inside grid
  records identically in debug and baked release. Inspect cartridge templates
  and generated descriptors rather than assuming no changes are needed.
- Runtime GPU conversion, retained hashing/invalidation, batch concatenation,
  paint blend descriptors, capture signatures, texture limits and cleanup.
- Piet must report unsupported mesh paint explicitly and paint nothing in v1;
  no radial or first-color fallback. This needs approval because Piet currently
  supports the other gradient paints. A real CPU mesh raster fallback can be
  scoped later, not hidden inside the GPU delivery.

Tests should cover anchor interpolation and shared-edge derivatives, premultiplied
alpha/zero-alpha behavior, clamp bounds, invalid grids, folds/ordering, affine
transforms and nonzero paint origins, shared-edge pixel seams, stable stroke
domains through reveal, nested clips/alpha masks/group opacity, mixed-paint
crossfades and interrupted transitions, atlas limits, resource reuse and cleanup.
Add rich/binary/rust-manifest/value roundtrips and a full web baked-release
fixture with reactive nested position/color bindings. Audit any default-color
or visibility heuristic so sparse/transparent meshes do not vanish incorrectly.

After approval, create `examples/src/mesh-gradients`: static/morphing CMY panels,
control-grid overlay, pause/resume, resize, direct gradient stroke and continuous
Handwriter-mask treatments, and alpha/opacity diagnostics. Test web WGPU debug
and baked release at desktop/mobile sizes; report actual macOS/iOS/Piet checks
separately. No native text/control mask capture or new material/light behavior.

Canonical builder docs belong in a subsection of `drawing-styling.md` with a
small example and limits, plus a deliberate link from `compositing-effects.md`.
Update Rust API source comments and generated references, example bundles as
applicable, and `design/pain-points.md`. Run focused tests, web builds,
`mdbook build pax-docs/book`, example sync checks, formatting and `git diff --check`.
This proposal alone changes no supported behavior and does not need new public
capability prose or a web build.

## Minimal future website integration

Keep the existing two-child `Mask alpha=true` and Handwriter source unchanged.
Replace the first child's temporary sliding-gradient rectangle with a
full-size unlit rectangle filled by a 3×3 mesh. Keep its boundary covering the
rectangle; gently animate interior anchors/colors from the visible hero's
existing clock. Pause/resume freezes that clock and offscreen work; the
Handwriter writing clock remains independent. Keep the native text equivalent
outside the source subtree. No lighting, extra masks, or copied glyph paths.

## Decisions requested before implementation

1. Approve the automatic bicubic colored-anchor grid and `@gradient { mesh:
   { rows: ... } }` API, bounded initially to 2–8 anchors per axis; defer explicit
   handles and arbitrary connectivity.
2. Approve encoded-sRGB premultiplied cubic interpolation with clamping,
   transparent outside-domain behavior, and deterministic replacement for folds.
   Whole-Paint transitions remain crossfades; point/color bindings do morphing.
3. Approve the retained paint-texture/sampler extension and internal prototype
   gate; no scene-compositor rewrite or silent approximation.
4. Approve WGPU-first support with an explicit unsupported Piet boundary,
   and native WGPU claims limited to what is actually tested.
