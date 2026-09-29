# PAX-1008 — Paint, fill layers, and stroke layers

Accepted implementation contract, September 28, 2026. The decisions below
include the approved 1px Stroke default and symmetric 1px paint domain for a
collapsed axis. The implementation and validation checkpoint is recorded below.

This supersedes the stroke-paint proposal in the
[component-mask checkpoint](PAX-1008-mask-sources-and-stroke-paint.md#gradient-stroke-proposal).
The completed component-source lifecycle and Scroller fixes remain independent.

## 1. Outcome and scope

A vector element can have zero or more fills and zero or more strokes. Each
layer owns its paint, material, and opacity. Each stroke also owns its width,
cap, and join. Simple color assignments remain concise; structured layers and
mixed lists expose additional control as needed.

The completed feature includes solid, linear-gradient, radial-gradient, and
interpolated paints on every supported fill and stroke layer. Multiple layers
are in scope for this pass.

| Consumer | Supported appearance |
| --- | --- |
| Rectangle, Ellipse, Path | Fill stack and stroke stack |
| Line | Stroke stack; no new interior-fill API |
| Handwriter | Stroke stack forwarded to its generated Path |
| Components built from those elements | Can expose and forward the same layer types |

Handwriter keeps its existing text-generation, accessibility, and selection
behavior. Its invisible native text equivalent does not acquire paint layers.
Native Text, native controls, and native container surfaces do not gain multiple
paints or per-layer materials in this work. Existing paint-valued adapters may
be mechanically migrated to the renamed Paint type while preserving their
current capabilities and limitations.

Excluded: blend modes, fill/stroke interleaving, stable layer IDs, automatic
layer matching, per-layer draw ranges, new stroke alignment/dash features,
image/video/pattern/shader paints, and arbitrary GPU source-subtree capture.
No website composition changes or documentation publication are implied.
The approved bounded alpha-source capture extension is specified in section 15.

## 2. Baseline and dependencies

The user-initiated rebase onto main `a9b7aa7b0` completed at `961faada2`.
It preserves the component-mask/Scroller fixes and incorporates main's shared
focal-radial sampling, premultiplied Paint interpolation, and PAX-1004 retained
opacity composition. Section 11.1 referred to this base integration. The rebase
conflicts were overlapping pain-point notes and the generated example source
bundle; both sets of notes were retained and the bundle was regenerated.

## 3. Public value model

Rename the current authoring `Fill` enum to `Paint`, preserving main's variants,
unit semantics, default color, and interpolation behavior:

```rust
pub enum Paint {
    Solid(Color),
    LinearGradient(LinearGradient),
    RadialGradient(RadialGradient),
    Blend(Vec<(Paint, f64)>),
}

pub struct Fill {
    pub paint: Property<Paint>,
    pub material: Property<Material>,
    pub opacity: Property<Opacity>,
    // Future blend_mode governs this layer's composition over lower layers.
}

pub struct Stroke {
    pub paint: Property<Paint>,
    pub material: Property<Material>,
    pub opacity: Property<Opacity>,
    pub width: Property<Size>,
    pub cap: Property<StrokeCap>,
    pub join: Property<StrokeJoin>,
    // Future blend_mode governs this layer's composition over lower layers.
}
```

The sketches omit derives and supporting implementations. Use the existing
Opacity type so `0.5` and `50%` mean the same thing. Finite opacity values
normalize to [0, 1]; nonfinite values must not reach GPU buffers and resolve to
zero opacity with the normal development diagnostic mechanism.

`Paint::Blend` is a weighted mixture sampled as one paint. It preserves the
existing paint-transition implementation and is distinct from ordered layer
composition and future blend modes. Do not turn a paint crossfade into several
ordinary source-over layers.

The canonical primitive properties keep their singular names:

```rust
pub fill: Property<Vec<Fill>>,
pub stroke: Property<Vec<Stroke>>,
```

Line and Handwriter expose only the applicable stroke property. Primitive-level
`material` is removed from the affected vector APIs; material lives on each
layer. There is no parallel `color`/`paint`, `fill`/`fills`, or element/layer
material precedence system.

Resolved renderer paint remains distinct from authored Paint. Rename internal
paint-valued types and adapters consistently where useful, without wrapping
them in the new Fill layer type. Gradient conversion, paint-mixture evaluation,
and alpha-paint records consume paint; an appearance layer additionally selects
geometry, material, and opacity.

## 4. Defaults and absence

| Situation | Meaning |
| --- | --- |
| Omitted Rectangle/Ellipse/Path `fill` | Preserve the existing default: one SLATE fill |
| Omitted primitive `stroke` | No strokes, preserving today's zero-width result |
| Omitted Handwriter `stroke` | One black 3px stroke with round caps and joins |
| `fill=[]` / `stroke=[]` | No layers in that stack |
| Bare fill paint | One Fill with that paint, default matte material, opacity 1 |
| Bare stroke paint | One Stroke with that paint, width 1px, butt cap, miter join, default matte material, opacity 1 |
| Explicit layer object | Shared layer defaults, overlaid by its supplied fields |

`Fill::default()` has `Paint::default()` (solid SLATE), `Material::default()`
(matte), and opacity 1. `Stroke::default()` adds width 1px, butt cap, and miter
join. This is a deliberate change to the old zero-width Stroke default: absence
is now represented by an empty stack. Migrate internal uses of
`Stroke::default()` that currently mean "do not stroke" to explicit absence.

Typed and untyped layer objects use the same defaults. For example,
`stroke=RED`, `stroke={paint: RED}`, and `stroke=Stroke {paint: RED}` all produce
the same 1px stroke. Supplying a stroke replaces Handwriter's default stack; it
does not merge fields with Handwriter's default stroke.

Stroke widths remain logical pixel sizes. Percentage widths are unsupported.
Nonpositive widths produce no stroke coverage; nonfinite widths produce no
coverage and a development diagnostic. Existing cap/join geometry is preserved.

## 5. Authoring and coercion

Normal tag attributes continue using `=`. The feature does not add a second tag
attribute grammar. Existing object/settings field syntax remains available.

```pax
<Rectangle fill=RED />
<Rectangle fill=[RED, BLUE] />
<Rectangle fill={paint: RED, opacity: 50%} />
<Rectangle fill=[Fill {paint: RED}, BLUE] />
<Rectangle fill=@gradient { 0%: RED, 100%: BLUE } />

<Rectangle fill=[
    @gradient { 0%: rgba(255, 255, 255, 0), 100%: WHITE },
    Fill {paint: RED, material: Material::Unlit},
    BLUE
] />

<Rectangle fill=[] stroke=[
    {paint: WHITE, width: 2px},
    {paint: @gradient { 0%: CYAN, 100%: MAGENTA }, width: 8px}
] />

<Rectangle fill=[
    Fill {paint: {self.accent}, opacity: {self.highlight_opacity}},
    RED
] />
<Rectangle stroke={self.outline_layers} />
```

Normalize at the expected type boundary:

1. A Color becomes `Paint::Solid`; existing Paint values retain their meaning.
2. A Paint becomes a default Fill or Stroke with that paint, according to the
   target property. A layer object constructs that layer using its defaults.
3. One valid layer value becomes a one-entry stack.
4. A flat list normalizes each entry independently into the target layer type.
   Shorthand paints, typed layer objects, untyped objects, and bound values can
   coexist. An empty list remains empty.

These rules apply to literals, reactive expressions/bindings, settings, and
supported timeline values in both rich and baked cartridges. Wrapped PAXEL
expressions retain their existing declarative semantics. Supporting template
values inside lists does not make `@gradient` an arbitrary PAXEL operator.

Singleton promotion is specific to appearance stacks, or an explicit opt-in
coercion facility. Do not change all `Vec<T>` conversions. If an implementation
wrapper is needed internally, the same normalization and vector behavior must
hold at every property boundary.

Lists containing nested gradients and reactive structured objects require
recursive value lowering. Preserve typed object metadata, dependency tracking,
and gradient definitions through the manifest, runtime evaluation, formatting,
and binary representation. Do not deserialize such a list eagerly into static
PaxValues and lose its live expressions.

Boundaries and diagnostics:

- No implicit flattening of nested lists and no implicit removal of `None`.
  Use an explicit empty list for absence.
- A Stroke cannot silently become a Fill, or vice versa. Preserve explicit type
  information sufficiently to diagnose a mismatched layer type.
- Unknown fields and obsolete `color` fields report an error; never ignore
  them. A failure reports the property, zero-based entry index where applicable,
  field, expected type, and source location when available.
- Static errors should be diagnosed before mounting. Dynamic invalid values
  follow the runtime's property-error policy without partial list application
  or divergent debug/release fallback. Add location context to that policy as
  needed; do not invent a second permissive conversion path.
- Syntax accepted as a direct layer must have the same semantics when nested
  in a one-entry list, including updates to reactive fields.

## 6. Ordering and composition

There are two independent ordered stacks. Every stroke is above every fill.
Index zero is topmost within each stack, matching Pax's element ordering.
Render fills back to front, then strokes back to front. Reordering a list
changes its visual order; no separate z-index is introduced for these layers.

For each layer:

1. Resolve its interior or stroked geometry and paint coordinate domain.
2. Sample its paint, including any paint mixture.
3. Apply its material using the element's lighting context.
4. Apply layer opacity and composite with normal source-over semantics.

Element opacity applies once to the completed fill-and-stroke appearance.
For two overlapping opaque layers, element opacity 0.5 yields alpha 0.5 in
their overlap. Two layers each at opacity 0.5, with element opacity 1, yield
alpha 0.75 there. Paint alpha, layer opacity, element opacity, and enclosing
composition boundaries each contribute exactly once at their respective stage.

Use retained composition from the integrated base when isolation is necessary.
Elide a capture only when direct rendering is equivalent. Handle ancestor
opacity boundaries, clips, masks, tiles, and Scroller coordinate spaces without
reapplying inherited opacity inside a captured surface. This changes the
affected vector element's opacity contract and requires a migration note.

No blend-mode field is exposed until its behavior is implemented. Keep the
extension comments on Fill and Stroke; source-over is the only layer
composition mode in this pass.

## 7. Paint coordinates and stroke progress

Resolve the paint domain from the complete local geometry after smoothing and
before stroke expansion or `draw_start`/`draw_end` trimming. All layers on that
geometry use the same domain. Include the bounds' origin when resolving
percentage or pixel endpoints. Local transforms then carry geometry and paint
together through translation, rotation, reflection, scale, and shear.

For Rectangle/Ellipse this is the underlying shape's local bounding box. For
Path it is the full smoothed centerline bounds, including all subpaths; for Line
it is the bounds of its complete centerline segment. Handwriter uses its entire
generated Path. Neither a partial reveal nor a change to stroke width, cap,
join, material, or layer order changes the domain.

Approved degenerate rule: expand each zero-extent axis symmetrically
to 1 logical pixel about its original coordinate. Leave nonzero axes unchanged.
This fallback is independent of stroke width and shared by all layers. It is
only a paint reference frame; it creates no additional coverage. An empty path
still paints nothing. Nonfinite geometry paints nothing with a development
diagnostic. Tests must keep near-degenerate calculations finite too.

Carry these paint bounds separately from trimmed paths, stroked outlines, and
screen-space coverage bounds. Piet must not re-resolve its brush against a
trimmed path; mask extraction must not derive paint bounds from the expanded
outline. Colors clamp/extend according to the existing Paint contract outside
the reference domain.

Reuse main's radial contract: `start` is the focal point, `end` is the outer
circle's center, and numeric `radius` is in local logical pixels. Equal points
produce concentric circles. Preserve main's invalid-input handling, pixel and
percentage stop semantics, and eight-stop GPU limit. Apply the same paint
mapping to fills, strokes, and their supported alpha-mask representations.
Any remaining linear transform discrepancy must be fixed at the shared paint
resolution/sampling boundary, with a regression for existing fills.

`draw_start`/`draw_end` remain element properties and affect every stroke layer
over the same complete path progression. Interior fills retain their current
behavior under a stroke reveal. Do not regenerate Handwriter glyphs when only
paint, material, opacity, stroke style, or reveal progress changes.

## 8. Reactive updates and transitions

Use main's existing paint interpolation: solid-to-solid interpolates RGBA;
other paint pairs crossfade sampled paints in premultiplied RGBA while retaining
each endpoint's geometry and stops. This includes solid/gradient, linear/radial,
and unequal stop counts. Reuse Material's established transition semantics.
Layer opacity interpolates as normalized alpha, width uses the supported Size
semantics, and cap/join switch at the end of a transition.

Stacks have positional identity. When source and destination lengths match,
interpolate entries at corresponding indices. A reorder therefore changes the
contents of those positions; it does not animate layer identity or z-position.
When lengths differ, switch to the destination stack immediately, matching
main's Vec interpolation. With no requested transition, updates take effect
normally. Fill and stroke stacks transition independently.

Replacing/removing a layer must release its subscriptions and retained render
resources. Mutating a nested paint/material/opacity field must dirty the owning
appearance correctly, including when used in a component mask source. Do not
require replacing the outer Vec merely to observe a live nested Property.

## 9. Renderer, coverage, and mask integration

Keep specialized fill and stroke geometry generation. They feed common paint
resolution and draw operations; strokes retain path-progress attributes needed
for efficient reveals. Additional fill layers reuse compatible geometry;
different stroke widths/caps/joins may legitimately require distinct geometry.

Paint/material/opacity-only changes must preserve geometry cache entries and
avoid glyph generation or tessellation. Reordering can change draw submission
order while reusing compatible geometry. Stationary appearances reuse retained
work. Count resource churn in tests or instrumentation before claiming these
properties; multiple layers still cost paint sampling and composition work.

Coverage bounds include every visible stroke's expansion and cap/join behavior.
Update opacity estimates to account conservatively for ordered layers and
element opacity. Never cull content behind an appearance on an unsupported
assumption that its combined coverage is fully opaque. Preserve the existing
hit-testing policy; this work does not introduce per-pixel hit testing.

Supported vector primitives used as alpha-mask sources contribute the alpha
of their complete layered appearance, including draw range, layer opacity,
paint alpha, and element opacity. Carry primitive composition boundaries in
the alpha representation or use equivalent shared rendering. Flattening all
layers and multiplying element opacity into each paint is incorrect.

Section 15 adds same-surface vector source capture and its Group/Frame/Mask
semantics. Cross-surface capture, native-source capture, and native content
inside alpha masks remain excluded. Preserve
geometric-mask semantics, including the existing full-stroke outline policy for
a nonempty draw range; alpha masks continue to follow the painted reveal.

Target WGPU web as the end-to-end acceptance chassis. Implement ordinary vector
paint/layer behavior in the Piet fallback too, including radial and mixed
paints and element opacity. Existing lack of Piet alpha-mask support remains
explicit. Platform availability may limit execution coverage, but it is not a
license for silent solid-color fallback or partially wired backend code.
Distinguish native platform targets from excluded native elements: shared GPU
code may run on Apple targets, whose runtime validation must be reported
separately from web validation.

## 10. Migration and compiler/runtime contract

This is an intentional API migration. Update repository consumers, examples,
templates, tests, and relevant bundled sources in the same implementation.

| Existing form | New form |
| --- | --- |
| `fill=RED` / `fill=@gradient {...}` | Unchanged authoring; becomes one Fill layer |
| `stroke={color: RED, width: 2px}` | `stroke={paint: RED, width: 2px}` |
| `Fill::Solid(color)` as a paint value | `Paint::Solid(color)` |
| Primitive `material=...` | Material on each intended fill/stroke layer |
| Rust single Fill/Stroke property | Ordered Vec of the new layer type |
| Internal zero-width default used as no stroke | Explicit absence/empty stack |

Moving an old primitive material requires applying it to each former drawing
operation that used it. Audit bindings and shared styles, not only literals.
Old `stroke.color`, paint-enum constructors under `Fill`, and removed primitive
material assignments receive actionable migration errors rather than ignored
fields. No dual runtime field or permanent compatibility precedence is added.

Audit defaulting, equality/hash, interpolation, ToPaxValue/coercion, type
descriptors, structured-property traversal and binding, public exports, and
static analysis. Register all layer fields, including cap/join; do not preserve
the older incomplete Stroke descriptor.

The recursive list values cross the compiler/runtime boundary. Update all
affected `ValueDefinition` handling, compiler collection/generation/templates,
runtime evaluation, `program_ir`, `binary`, `rust_manifest`, and source
serialization/formatting. Keep source-only metadata out of baked execution
data. If the wire format changes incompatibly, bump the relevant version and
reject stale artifacts clearly; applications rebuild with the matching toolchain.

Ordinary Color/Paint bindings, mixed lists, nested reactive fields, and settings
transitions must agree between rich debug execution and baked release execution.

## 11. Implementation sequence

1. Integrate the landed base through Zack's Git workflow and run the existing
   component-mask/Scroller regression suite on that base.
2. Rename paint types; introduce Fill/Stroke layers, defaults, complete type
   metadata, and targeted stack coercion. Implement recursive list lowering and
   baked/source roundtrips before wiring only the easy literal cases.
3. Migrate vector consumers and repository usage. Render ordered fill and
   stroke stacks with per-layer material, shared paint domains, and full Paint
   parity. Keep each incremental change buildable.
4. Implement element-opacity boundaries, alpha-source appearance parity,
   conservative coverage, nested-property invalidation, and resource cleanup.
5. Complete acceptance fixtures, performance checks, public documentation,
   generated API references, and target/backend verification.

These are reviewable implementation steps within one feature. Multiple layers
and full Paint parity are part of completion, not deferred after shipping the
new public model. Bounded vector source capture is the approved follow-on in section 15.

## 12. Acceptance criteria

### Language and serialization

- Compile every authoring form in section 5 and assert equivalent normalized
  values for shorthand, explicit typed/untyped objects, and one-entry lists.
- Exercise empty lists, heterogeneous shorthand entries, direct/nested
  gradients, bound single Paint/layer/Vec values, and nested reactive fields.
- Check descriptive failures for nested lists, `None`, wrong explicit layer
  types, unknown fields, and obsolete API names. No partial stack installation.
- Roundtrip source formatting, rich manifests, program IR, and baked data;
  execute a reactive mixed-list fixture in debug and release.

### Rendering and lifetime

- Verify first-entry-on-top order within both stacks and strokes above fills;
  include independently sized strokes, caps/joins, and distinct materials.
- Check unchanged omitted-property defaults, explicit empty stacks, 1px
  shorthand strokes, and Handwriter's omitted 3px round stroke.
- Pixel-check linear/radial/solid/mixture parity on fills and strokes, including
  transparent stops, shifted path origins, rotation, reflection, shear,
  nonuniform scaling, and all documented radial cases.
- Check stable paint under reveal and stroke-width animation, plus horizontal,
  vertical, collapsed, smoothed, and multi-subpath geometry.
- Distinguish the 0.5 element-opacity overlap result from the 0.75 per-layer
  result. Include nested opacity boundaries, clipping, and Scroller surfaces.
- Verify supported primitive alpha-source appearance against visible alpha;
  preserve geometric-mask and unsupported-source boundaries explicitly.
- Exercise positional transitions, immediate length changes, direct reorder,
  layer removal/replacement, template hot reload, and full disposal.
- Verify coverage/occlusion does not hide underlying content incorrectly.

### Performance and targets

- Paint/material/opacity-only updates and GPU reveal progress produce no new
  glyph geometry or stroke tessellation after warm-up. Compatible geometry is
  reused across layer changes. Retained captures are reused when only their
  presentation opacity changes; removed resources retire correctly.
- Extend the `mask-strokes` fixture with direct layered strokes/fills while
  retaining its component-source and nested-Scroller regressions.
- Run focused language, runtime API, descriptors, manifest/baking, runtime,
  standard-library, and GPU tests. Exercise WGPU web debug and baked release at
  desktop and narrow widths. Validate the Piet vector path where available and
  report Apple-target execution coverage precisely.
- Finish with formatting, `git diff --check`, API generation, and the book build.

## 13. Documentation deliverables

Update the public learning path with the implementation, without presenting
the API without implying a published release:

- [Drawing and Styling](../drawing-styling.md): Paint versus layer types,
  progressive authoring, stack order/defaults, per-layer materials and opacity,
  coordinates, gradient strokes, and migration.
- [Data Binding and Expressions](../data-binding-expressions.md): targeted
  coercion, mixed lists, nested reactive objects, and diagnostics.
- [Animation and Motion](../animation-motion.md): positional stack transitions,
  changed-length behavior, and retained paint crossfades.
- [Compositing and Effects](../compositing-effects.md): complete-appearance
  opacity and supported layered alpha-source behavior/limits.
- [Text, Fonts and Images](../text-fonts-images.md) and affected API comments:
  mechanical Paint renames and explicit native-element limitations.
- Canonical example sources/README, public Rust comments and generated API
  output, relevant bundled templates, and newly discovered pain points.

Preserve useful anchors and avoid adding a public chapter for this internal
specification. No CLI syntax change is planned; review first-touch examples and
README snippets for renamed stroke/material forms. Documentation publishing and
changes to released snapshots require separate authorization.

## 14. Paint-layer implementation checkpoint (before capture integration)

Implemented Paint plus ordered Fill/Stroke stacks, recursive reactive list
lowering and rich/baked serialization, static/dynamic layer validation, WGPU and
Piet vector painting, shared stroke paint domains, primitive alpha-source
composition, nested-property invalidation and retirement, and repository/API
migrations. Rich manifest binary is version 5; baked ProgramIR is version 4.

The extended `mask-strokes` fixture passes WGPU web debug and baked release at
desktop and 390px widths. Paint/reveal animation, stack reversal, removal and
restoration, nested live paint, opacity comparisons, and Scroller mask output
were inspected. Template hot reload rebuilt the layered study successfully.
The migrated website also compiles and renders its existing hero/handwriting;
this does not introduce new website mask compositions.

Validation includes 640 integrated CPU tests and 16 Metal-backed GPU tests.
New pixel tests compare fill/stroke gradients, layer order, element/layer alpha,
and mask paint changes; counters show no new tessellation for warmed paint
updates or visible GPU stroke reveals. Alpha-source reveals still change their
CPU-extracted outlines; that existing path rebuilds geometry as the reveal
changes. Section 15 supersedes this outline-extraction limitation by sharing
ordinary vector source rendering. Subscription churn returns to the original property count.
API reference generation, source bundles, and mdBook build are part of this
checkpoint. Native macOS/iOS applications have not been launched; Metal tests
exercise the renderer, not complete Apple chassis integration. Arbitrary source
subtree capture, native paint stacks, blend modes, and stable layer IDs remain
outside the accepted scope.


A forced-Piet, vector-only browser fixture also passed linear/radial stroke,
mixed-paint stroke, ordering, and opacity checks. The alpha-mask portions are
excluded from that fixture because Piet rejects alpha masks explicitly.
Five clock-driven integration tests and nine API/source-bundle tooling tests
passed in addition to the suites above. The 69 migrated/new templates parse;
legacy template formatting was retained to keep the migration diff focused.
Rust formatting and `git diff --check` pass, and 1,582 local links across changed
book pages resolve, including their anchors. Feature changes are uncommitted.


## 15. Approved retained alpha-source integration

Zack approved this extension on September 28. Connect alpha-mask sources to the
retained subtree surfaces landed in PAX-1004; do not build another vector painter
or weaken the reveal-performance acceptance criterion.

- Record detached vector sources under explicit mask ownership. Keep their
  component lifecycle, but never add their draws to visible ordering, hit testing,
  or native presentation. Preserve source-local unclippable ordering without
  letting source draws escape into the visible scene.
- Share retained capture allocation, nesting, cropped texture storage, content
  signatures, and retirement with group opacity. The opacity consumer samples
  RGBA; the mask consumer extracts alpha and optionally feathers it.
- Render existing vector primitives and their layers through ordinary geometry,
  paint, material, draw-range, and opacity-scope paths. Preserve source-side
  vector Frame and Mask clips, including nested source masks. Strip consumer
  ancestry from source opacity; apply enclosing alpha after capture/feathering.
- Use an explicit surface-local capture domain with sufficient padding for
  nested feather kernels. Stage projection changes in GPU command order; keep
  ordinary and source stencil attachments independent. Over-limit domains fail
  closed with a diagnostic. A coordinate-only change must not reallocate
  unchanged-size mask textures or upload buffers.
- Keep image/native sources, source-side Scrollers, cross-surface capture, and
  Piet alpha masking excluded. Geometric masks keep coverage extraction.
- Verify source/visible alpha parity for transformed gradient and mixed-paint
  strokes, 50%-opacity group overlap, nested clips/masks, source disposal, and
  feathering at tile edges. Warmed paint and reveal updates must not tessellate.
  Rebuild and inspect WGPU web debug and baked release at desktop/narrow widths;
  retain the existing Scroller and live-component regressions.

This changes the source-production path, not the content-side alpha-mask
contract: alpha still modulates individual canvas draws. No new public
Paint/Fill/Stroke authoring API or manifest/baked value shape is introduced.

### Integration checkpoint

Implemented shared retained captures for ordinary opacity groups and detached
vector alpha sources. Source rendering now uses ordinary primitive hooks,
layered paint, GPU stroke ranges, opacity scopes, and vector clips. Captures are
cropped and cached, and their draws, textures, and stencil resources retire when
the source is no longer referenced. Padded domains preserve feathering across
tile edges and nested masks; an oversized source fails closed independently of
other masks.

Validation passes 642 integrated CPU tests and 20 Metal-backed GPU tests. The
new pixel regressions cover transformed linear/radial/mixed-paint strokes,
source group-opacity overlap, nested masks and clips, feathering beyond a tile,
empty/oversized sources, and resource retirement. Counter assertions establish
no retessellation during warmed source stroke reveals and reuse of unchanged
capture pixels.

The expanded `mask-strokes` fixture builds and renders in WGPU web debug and
baked release at desktop and 390px widths. Visible/captured opacity comparisons,
nested feathered sources, live handwriting, source removal/remounting, and
nested Scroller alignment were inspected. Documentation, generated API pages,
and example source bundles are updated. Native macOS/iOS applications remain
unlaunched; image/text/native sources, source-side Scrollers, cross-surface
capture, and Piet alpha masks remain outside this integration.

Five clock-driven integration tests and the documentation/tooling tests also
pass. API generation, both source-bundle freshness checks, mdBook, Rust
formatting, and `git diff --check` pass; 1,596 local links across 21 changed book
pages resolve, including anchors. Changes remain uncommitted.

### Apple debug checkpoint (September 29)

Native macOS and an iPad Pro 13-inch (M5), iOS 26.4 simulator now build and run
the fixture. Both pass the visible paint-layer studies, capture/group-opacity
comparison, nested feathered source, and native paint pause/reversal controls.
macOS additionally passes live handwriting, source removal/remounting, and
outer/nested scrolling. Fixture Buttons were corrected to use `button_click`
with `Event<ButtonClick>`; browser `click` activation had hidden that portability
mistake.

iPad swipe/scroll attempts through native automation did not move the page.
Zack subsequently verified the lower handwriting/source-lifecycle/nested-scroll
checks manually. No Apple release build or physical-device validation is claimed.
This supersedes the earlier statement that Apple applications had not been launched.

### Main rebase checkpoint (September 29)

Rebased onto `36e6b770b`, retaining main's shared geometry, incremental native
compositing, light-scope indexing, and viewport-proximity behavior. Detached
mask sources reuse cached geometry without joining visible spatial indexes or
receiving viewport events. Changes reconcile their visible mask owner through
nested source ownership, preserving incremental compositing during animation.

Vector primitives now retain and evaluate their nested appearance subscriptions.
A regression changes a source stroke's width without replacing its layer stack
and verifies that cached bounds refresh. Additional regressions cover detached
viewport exclusion and incremental reconciliation.

Post-rebase validation passes 453 core library tests, 193 compiler tests, 28
clock/geometry/viewport integration tests, and 15 Metal retained-clip tests.
Compiler WebSocket tests were rerun with localhost binding allowed after the
sandbox blocked five binds; all 12 WebSocket tests pass. The manual timing test
remains ignored. API generation, example-bundle freshness, mdBook, focused Rust
formatting, and diff whitespace checks pass. Workspace-wide formatting reports
only three unchanged files inherited from main.

The final web debug and baked-release artifacts build and render correctly.
Paint pause/reversal, visible/captured opacity parity, nested feathering, and
source removal/remounting pass in both. The baked release also passes nested
and outer scrolling and the 390px layout check. Apple debug verification above
predates this rebase; Apple applications were not rerun after it. Rebuilt caches
are retained for continued review.
