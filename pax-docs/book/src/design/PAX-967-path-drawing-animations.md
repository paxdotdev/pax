# PAX-967 Path Drawing Animations

Status: implementation handoff draft
Last revised: 2026-07-06

<!-- summary: API and implementation spec for path drawing / stroke reveal animations and SVG-backed path components. -->
<!-- tags: animation, drawing, path, stroke, svg, timeline, runtime -->

## Summary

Add first-class path drawing support to `Path` by exposing normalized
`draw_start` and `draw_end` properties. These properties trim the rendered
stroke by path length while leaving existing `Path` output unchanged by default.

Add a compile-time SVG component source attribute:

```rust
#[pax]
#[svg("design-assets/logo-mark.svg")]
pub struct LogoMark {}
```

`#[svg(...)]` is the primary SVG import API. The Rust item remains the component
declaration: removing, renaming, exporting, or changing the struct changes the
Pax component in the same way as existing `#[file("...")]` components.

Provide `pax-cli svg-import` as support tooling for validation, debugging, test
fixtures, and explicit ejection into editable Pax source. Static generation is
not the default authoring flow.

## Problem

Pax can animate ordinary properties through timelines, PAXEL expressions, and
imperative `Property::ease_to`, but `Path` does not have a first-class way to
reveal a stroke as if it is being drawn over time.

The motivating use case is handwriting or pen-like drawing: an author imports
or authors centerline path data, or stroke-only outline contours, and animates a
playhead so the stroke appears from the start of the path to the end.
Illustrator-exported SVG is expected to be an important source format for those
paths.

This feature is adjacent to, but distinct from, PAX-943 "animate along path":

- PAX-967 reveals some portion of a vector path.
- PAX-943 moves some other subtree along a path.

Both features may eventually share path measurement utilities, but their
authoring surfaces should remain separate.

## Goals

- Preserve current rendering by default for every existing `Path`.
- Make the common handwriting case a one-property timeline.
- Support declarative timelines, PAXEL expressions, bindings, and imperative
  easing through normal Pax property mechanics.
- Support `f64`-style unitless values and percent values for drawing progress.
- Support dynamic path geometry, including paths authored with existing
  `PathPoint`, `PathLine`, `PathCurve`, and `PathClose` children.
- Use real path length, not segment-count interpolation, for lines, quadratic
  curves, cubic curves, and close-path segments.
- Keep the path trim renderer backend-neutral where possible.
- Treat SVG import as a compile-time component-template source via `#[svg]`.
- Keep generated Pax source available for debugging and ejection without making
  it the normal source of truth.

## Non-goals

- Runtime SVG import.
- General SVG rendering. The import phase should start with a limited,
  deterministic subset useful for Illustrator path data.
- Motion along a path; that belongs to PAX-943.
- Revealing arbitrary subtrees or mixed vector/native content in the first pass.
- A filled-shape wipe/mask API in the first pass.
- Morphing between two `Vec<PathElement>` values.
- New timeline syntax or a new animation controller API.
- Unprefixed path child tags such as `<Point>` or `<Curve>`. The current
  prefixed path-child API is acceptable.

## Existing Pax Surface

`Path` is a `pax-std` primitive with these public properties:

- `elements: Property<Vec<PathElement>>`
- `stroke: Property<Stroke>`
- `fill: Property<Fill>`
- `material: Property<Material>`

`PathElement` currently supports:

- `Empty`
- `Point(Size, Size)`
- `Line`
- `Quadratic(Size, Size)`
- `Cubic(Size, Size, Size, Size)`
- `Close`

At runtime, `PathInstance` resolves `PathElement` values into a local
`kurbo::BezPath`, clips to the primitive bounds, fills the path, and strokes the
path through the backend-neutral `RenderContext`.

`Path` can also receive path-builder children. `PathPoint`, `PathLine`,
`PathCurve`, and `PathClose` write live `PathElement` values into a local
`PathContext`, and the parent recomputes flattened projected children when those
children change. This already gives PAX-967 the dynamic-path hook it needs.

The child API does not currently expose a `PathCubic` child. Because SVG paths
commonly use cubic commands, generated/ejected Pax for SVG import should use
`elements=[PathElement::...]` unless or until a cubic path-child component is
added.

## Proposed Path API

Add a reusable semantic type for values in a normalized unit domain:

```rust
pub enum UnitValue {
    /// Unitless value, where 0.5 means halfway through the domain.
    Unitless(Numeric),
    /// Percent value, where 50% means halfway through the domain.
    Percent(Numeric),
}
```

`UnitValue` should accept both numeric and percent Pax values:

- `0.5` coerces to `UnitValue::Unitless(0.5)`.
- `50%` coerces to `UnitValue::Percent(50)`.
- A resolver such as `to_unit_float()` returns normalized values:
  - `UnitValue::Unitless(0.5)` resolves to `0.5`.
  - `UnitValue::Percent(50)` resolves to `0.5`.
- A clamped resolver such as `to_clamped_unit_float()` clamps into `[0.0, 1.0]`.
- Interpolation should resolve both endpoints into normalized unitless values
  and return `UnitValue::Unitless(...)`, mirroring the existing `Opacity`
  interpolation pattern.

Then add two properties to `Path`:

```rust
pub struct Path {
    pub elements: Property<Vec<PathElement>>,
    pub stroke: Property<Stroke>,
    pub fill: Property<Fill>,
    pub material: Property<Material>,
    pub smoothing: Property<PathSmoothing>,
    pub draw_start: Property<UnitValue>,
    pub draw_end: Property<UnitValue>,
}
```

The non-zero `draw_end` default is required for backwards compatibility.
Convert `Path` to `#[custom(Default)]` and define:

```rust
draw_start = UnitValue::Unitless(Numeric::F64(0.0))
draw_end = UnitValue::Unitless(Numeric::F64(1.0))
```

Adding the fields without a custom default would make existing paths render with
`draw_end = 0.0`, which would be a visual regression.

### Path Smoothing

Add an optional `PathSmoothing` property for paths whose source data represents
curves as dense polylines:

```rust
pub enum PathSmoothing {
    None,
    Light,
    Strong,
}
```

`None` is the default and must preserve authored geometry exactly. `Light` and
`Strong` are intentionally coarse quality levels that can be extended later
without committing to a numeric smoothing parameter too early.

Smoothing should be a geometry-cache input, not a draw-range input. In the WGPU
renderer, include the smoothing level in the retained geometry signature and
perform smoothing immediately before tessellation. Do not resmooth every frame
when only `draw_start` / `draw_end` changes. The draw range should remain
primitive data so path drawing animation can update without rebuilding smoothed
geometry.

The occlusion/native-mask pass should also treat `draw_start` and `draw_end` as
render-only properties by default. It should use a conservative full-footprint
path for `Path` occlusion instead of recomputing stroke coverage for every
trimmed frame. This may over-mask native surfaces while a path is only partially
drawn, but it keeps path drawing animation fast and matches the first-pass
implementation, which does not provide true progress-sensitive clipping.

`Handwriter` should expose the same `smoothing: Property<PathSmoothing>` and
forward it to the internal `Path`. This lets authors use the bundled
single-stroke SVG fonts without choosing separate pre-smoothed font assets.

## Path Draw Semantics

- `draw_start` and `draw_end` are unit-domain positions over total path length.
- The visible stroke range is `[draw_start, draw_end]`.
- Authors may write unitless values such as `0.5` or percent values such as
  `50%`.
- Resolved values are clamped into `[0.0, 1.0]`.
- `draw_end = 0.0` or `draw_end = 0%` draws none of the stroke.
- `draw_end = 1.0` or `draw_end = 100%` draws the full stroke.
- `draw_start = 0.25, draw_end = 75%` draws the middle half of the path.
- If `draw_start >= draw_end`, no stroke is drawn.
- The properties affect stroke rendering and stroke coverage. They do not trim
  fill in the first pass.

Example:

```pax
<Path
    stroke={color: BLACK, width: 4px, cap: StrokeCap::Round}
    fill=TRANSPARENT
    draw_end=@timeline {
        duration: 1200ms,
        loop: false,
        0%: 0%,
        100%: 100%,
    }
>
    <PathPoint x=8% y=65% />
    <PathCurve x=30% y=20% />
    <PathPoint x=55% y=52% />
    <PathCurve x=78% y=82% />
    <PathPoint x=94% y=24% />
</Path>
```

### Multiple Subpaths

The trim range is measured over the path's resolved segment stream in authored
order. Move commands do not contribute length. The trim can cross `MoveTo`
boundaries, producing multiple visible subpath fragments when needed.

This matches the handwriting use case better than normalizing each subpath
independently: a single playhead advances through the whole authored gesture.

### Closed Paths

`PathElement::Close` becomes an ordinary drawable segment from the current point
to the current contour start. It contributes to total length and can be
partially drawn.

First-pass behavior does not wrap when `draw_start > draw_end`, even for closed
contours. Wraparound trim can be added later if real examples need chasing
segments around a closed loop.

### Fill

The first pass leaves fill unchanged. Authors who want a pen-writing effect
should use `fill=TRANSPARENT` or duplicate a filled path underneath/above the
drawn stroke.

Trimming fill along path length is not a well-defined vector operation for
closed shapes. A future wrapper or mask API is a better place for progressive
filled-art reveal.

### Hit Testing And Occlusion

Stroke coverage should use the trimmed centerline before `stroked_outline_path`
is computed. Otherwise a visually hidden part of the stroke could still affect
occlusion or hit testing.

Fill coverage remains based on the full path because fill remains untrimmed.

### Animation

No new timeline syntax is needed. Authors can use:

- inline timelines on `draw_end`
- named timeline selector tracks targeting `draw_end`
- expressions such as `draw_end={self.progress}`
- `Property<UnitValue>` values from Rust
- imperative `ease_to` from Rust

## SVG Component API

Use `#[svg("...")]` to declare a component backed by SVG source:

```rust
use pax_kit::*;

#[pax]
#[svg("design-assets/signature.svg")]
pub struct Signature {
    pub draw_start: Property<UnitValue>,
    pub draw_end: Property<UnitValue>,
}
```

API rules:

- `#[svg(...)]` is a component template source, analogous to `#[file("...")]`.
- The Rust struct is the component declaration and the component's typed prop
  surface.
- `#[svg]` is mutually exclusive with `#[file]`, `#[inlined]`, and
  `#[primitive]`. Follow the same convention used by the existing `#[file]` vs.
  `#[inlined]` check: the `#[pax]` macro should reject invalid combinations at
  compile time with a clear diagnostic before static analysis or manifest
  generation can proceed. Static analysis should keep a matching defensive check
  so non-macro or malformed inputs fail deterministically.
- `#[svg]` should be supported on `#[pax]` component structs. Do not add enum
  support unless a concrete existing Pax pattern needs it.
- The path string should resolve with the same project lookup rule as
  `#[file]`: relative to the Cargo manifest directory, then relative to `src`
  for compatibility. Absolute paths may be accepted if the current codebase
  already accepts them through canonicalization.
- The `#[pax]` macro must recognize and strip `#[svg]` just as it does for
  `#[file]`; otherwise Rust will see an unknown inert attribute after expansion.
- Static analysis must parse `#[svg]`, translate the SVG into an equivalent
  component template, and include that template in the manifest.
- The release cartridge path must receive the same baked component template as
  debug/source-linked builds.

The imported SVG file is source input, not a runtime asset dependency. Runtime
hosts should not load or parse the SVG for this feature.

### Compile-Time Expansion Model

`#[svg]` should not expand the SVG inline into runtime template data inside the
proc macro. The runtime-facing intermediate should follow the same manifest and
cartridge flow as existing Pax templates:

```text
Rust source with #[svg("...")]
        |
        | pax-cli static analysis / manifest build
        v
SVG source translated into Pax template data
        |
        v
ComponentTemplate stored in PaxManifest
        |
        v
.pax/cartridge.partial.rs
        |
        | included by #[pax] on the main component
        v
runtime init_manifest() returns PaxManifest
```

The durable intermediate between compile-time SVG import and runtime is the
`ComponentTemplate` inside `PaxManifest`, then the generated cartridge snippet.
Debug builds may embed that manifest as JSON in `cartridge.partial.rs`; release
builds may embed it as generated Rust. In both cases, runtime consumes manifest
data, not the source SVG and not a persistent generated Pax file.

The `#[pax]` proc macro still has responsibilities for `#[svg]`:

- parse, validate, and strip the `#[svg]` attribute
- enforce the same compile-time mutual-exclusion convention used for
  `#[file]` vs. `#[inlined]`
- classify the component as template-backed rather than struct-only
- emit a Cargo dependency hook, such as an `include_str!`-backed constant, so SVG
  file changes invalidate macro expansion and rebuild the component

Static analysis is the source of truth for the runtime template. It should read
the SVG, translate it, and assemble the component's `ComponentTemplate` in the
manifest. The translation library may internally produce raw Pax or a normalized
SVG/Pax IR first, especially to support `pax-cli svg-import --stdout`, but that
raw Pax is ephemeral unless the author explicitly ejects it with
`pax-cli svg-import --out`.

### SVG To Pax Mapping

The first `#[svg]` implementation should support deterministic SVG path import,
not arbitrary SVG rendering.

Required first-pass support:

- `<svg viewBox="min_x min_y width height">`
- `<path d="...">`
- path commands `M`, `L`, `H`, `V`, `Q`, `C`, and `Z`, plus their lowercase
  relative variants
- path-level `fill`, `stroke`, `stroke-width`, `stroke-linecap`, and `opacity`
  when they map cleanly to Pax `Fill`, `Stroke`, and opacity/material behavior
- simple group and path transforms that can be flattened into path coordinates
  before emitting Pax data
- SVG painter order translated into Pax z-order

Important z-order note: SVG paints later elements on top of earlier elements.
Pax renders earlier template nodes on top of later template nodes. The importer
must account for this by reversing sibling order when emitting Pax nodes, or by
otherwise preserving visual stacking.

Path command mapping:

- `M` / `m` move commands become `PathElement::Point`.
- `L` / `l`, `H` / `h`, and `V` / `v` line commands become
  `PathElement::Line` plus a following point.
- `Q` / `q` commands become `PathElement::Quadratic` plus a following point.
- `C` / `c` commands become `PathElement::Cubic` plus a following point.
- `Z` / `z` commands become `PathElement::Close`.

The importer should normalize relative commands into absolute coordinates before
building Pax path data.

### SVG Draw Control

The core guarantee is that imported SVG paths become Pax `Path` nodes, so they
can use `draw_start` and `draw_end` once represented in Pax.

There are two important first-use workflows:

- Centerline handwriting paths, where the imported path is the intended pen
  centerline.
- Stroke-only outline tracing, where block text or other shapes are represented
  by closed outline contours with empty fill and a visible stroke.

Both workflows should be supported by the same `draw_start` / `draw_end`
semantics. Closed outline contours are valid drawable paths: `PathElement::Close`
contributes length, and multiple contours can be represented as subpaths in one
Pax `Path`.

For the first `#[svg]` implementation, avoid a broad prop-forwarding system.
It is acceptable to generate static SVG-backed components that render fully by
default. To support the PAX-967 handwriting case without ejection, the importer
may implement this narrow convention:

- If the SVG component declares `draw_start: Property<UnitValue>`, bind that
  field to generated stroked `Path` nodes' `draw_start`.
- If the SVG component declares `draw_end: Property<UnitValue>`, bind that field
  to generated stroked `Path` nodes' `draw_end`.
- Do not bind these fields to filled-only paths.
- Compatible same-style stroked SVG paths should share one global draw domain,
  not independent parallel `draw_end` domains. The first implementation should
  prefer merging compatible paths into one compound Pax `Path` with multiple
  subpaths so a single `draw_end` playhead advances through the outlines in
  authored order.
- Do not bind the same component `draw_end` directly to several compatible
  generated paths if that would make every letter or contour reveal in parallel.
  If compatible paths cannot be merged, either remap the component's global draw
  range into each generated path's local range by cumulative length, or emit a
  clear diagnostic until remapping is implemented.
- Do not merge across incompatible fill, stroke, opacity, or transform contexts.
- Preserve SVG/authored path order for the first pass. Automatic spatial
  reordering, such as left-to-right glyph ordering, is a separate import option
  and should not be the default.

This convention is intentionally narrow and tied to PAX-967. More general prop
hooks for SVG imports should wait for concrete use cases.

Illustrator caveat: text converted to filled glyph outlines is not the same as a
handwriting centerline. That is still useful for outline tracing if the imported
artwork uses visible strokes and empty fill. A true pen-writing effect needs
centerline-like paths, while block-text outline drawing needs stroke-only closed
contours and a single sequential draw domain across compatible paths. The
path-drawing example should document both authoring conventions explicitly.

## Accessibility

`Path` is vector geometry and does not currently create a native accessibility
object. A future general a11y API should likely be common across primitives
rather than path-specific.

`Handwriter` can provide a useful first step because it owns the source text.
Expose `alt_text: Property<String>` and render an invisible native `Text` node
under or over the stroked path. When `alt_text` is empty, use `text`. This gives
screen readers, crawlers, and text-selection machinery real text while the
visible handwriting remains vector geometry. Also expose
`selectable: Property<bool>` with the same meaning and default as `Text`, so
authors can opt out of the invisible selection layer for non-selectable
handwriting. Alignment does not need to be perfect in the first pass; the key
foundation is preserving semantic text in the native layer.

The same pattern should not be blindly copied to arbitrary `Path` nodes, because
most paths do not represent text. For `Path`, prefer a future explicit
accessibility label/role API that can be implemented consistently across web,
native Apple, Android, and desktop chassis.

## Filled Shape Reveal

Animating fills with `draw_start` / `draw_end` is not well-defined. Stroke
drawing follows arc length along a centerline; fill drawing asks how an area is
painted over time. Closing an open partial contour with a straight edge usually
looks like mathematical extrusion, not paint.

Promising follow-up approaches:

- Clip or mask the filled shape with a moving brush/stencil driven by path
  length, so the interior feels painted instead of linearly extruded.
- Rasterize or tessellate full fill geometry once, then reveal it through a
  render-side coverage mask.
- Support author-provided reveal paths separate from fill outlines, especially
  for Illustrator workflows.

The classic public-domain Tiger SVG is a good stress test for this follow-up:
it has many filled contours, varied colors, and enough complexity to reveal
whether the model scales beyond simple glyphs. This should remain separate from
PAX-967 stroke drawing until a concrete fill-painting semantic is selected.

## `pax-cli svg-import`

Do not introduce nested `pax-cli svg import` initially. Until there are multiple
real SVG commands, use a single top-level command:

```sh
pax-cli svg-import design-assets/signature.svg --component Signature
```

The command should share the same SVG translation library as `#[svg]`.

Support three modes:

1. Normal validation/debug mode

   Validate the SVG against the supported subset and print diagnostics. With a
   `--stdout` or equivalent flag, print the generated Pax template to stdout.
   This is useful for tests and implementation debugging.

2. Test fixture mode

   Tests may invoke the same translation library directly or through
   `pax-cli svg-import --stdout`. Golden tests should compare deterministic
   generated Pax or normalized intermediate path data, not screenshots alone.
   Screenshots are still useful for end-to-end visual fixtures.

3. Eject/edit mode

   With `--out`, write an editable Pax component backed by `#[file]`, not
   `#[svg]`.

   Example:

   ```sh
   pax-cli svg-import design-assets/signature.svg \
       --component Signature \
       --out src/signature
   ```

   This may write:

   ```text
   src/signature.rs
   src/signature.pax
   ```

   The generated Rust should look like ordinary Pax component source:

   ```rust
   use pax_kit::*;

   #[pax]
   #[file("signature.pax")]
   pub struct Signature {
       pub draw_start: Property<UnitValue>,
       pub draw_end: Property<UnitValue>,
   }
   ```

   The generated Pax should include provenance comments:

   ```text
   // @generated by pax-cli svg-import
   // source: ../../design-assets/signature.svg
   // source_sha256: ...
   ```

Once the author edits ejected Pax, it is no longer an SVG import. It is a normal
Pax component with SVG provenance.

## Implementation Plan

### Phase 1: Path Draw Properties

1. Add `UnitValue` to `pax-runtime-api` with:
   - numeric and percent Pax coercions
   - `ToPaxValue` / `ToFromPaxAny` support as needed by current property
     plumbing
   - display/debug formatting
   - interpolation
   - normalized and clamped resolver helpers
2. Add `draw_start` and `draw_end` to `Path`.
3. Convert `Path` to `#[custom(Default)]` and define backwards-compatible
   defaults.
4. Include both properties in `PathInstance::handle_mount` dirty dependencies.
5. Add a path utility that returns a trimmed `kurbo::BezPath`:
   - compute total length from `path.segments()`
   - resolve `UnitValue` start/end values to clamped unit floats
   - convert resolved start/end into absolute arc-length values
   - walk segments in order
   - copy whole segments inside the range
   - use `PathSeg::inv_arclen` and `PathSeg::subsegment` for partial segments
   - begin each emitted fragment with `move_to(segment.start())`
6. In `PathInstance::render`, pass the full path to fill and the trimmed path to
   stroke.
7. In `PathInstance::resolve_coverage_path`, use the trimmed path for stroke
   coverage and the full path for fill coverage.
8. Add focused tests for `UnitValue`:
   - `0.5` and `50%` resolve to the same value
   - out-of-range unitless and percent values clamp at resolution
   - interpolation between unitless and percent endpoints is continuous
9. Add focused tests for path trimming:
   - empty path
   - line partial trim
   - trim crossing line boundaries
   - quadratic/cubic partial trim
   - close-path contribution
   - clamping and `draw_start >= draw_end`
10. Add a small example, likely `examples/src/path-drawing`, that demonstrates:
    - a simple handwritten stroke
    - stroke-only block-text outline tracing with closed contours
    - an inline timeline
    - a slider or property-bound playhead for manual inspection

Because these fields are runtime-visible properties on a primitive, audit
manifest generation, binary baking, release cartridge generation, and manifest
roundtrip tests as part of implementation.

### Phase 2: `#[svg]` Component Source

1. Extend the `#[pax]` macro config parser to accept and strip `#[svg("...")]`.
2. Extend static analysis config parsing to accept `#[svg("...")]`.
3. Enforce mutual exclusivity with `#[file]`, `#[inlined]`, and `#[primitive]`
   in the same compile-time validation path used for `#[file]` vs.
   `#[inlined]`, plus a matching defensive check in static analysis.
4. Add an SVG translation library used by both static analysis and
   `pax-cli svg-import`.
5. Translate supported SVG into a deterministic Pax component template.
6. Preserve source provenance and useful diagnostics:
   - unsupported element/attribute
   - unsupported path command
   - malformed path data
   - transform that cannot be flattened
   - missing or invalid viewBox
7. Ensure debug builds and baked release cartridges see the same generated
   component template.
8. Add fixtures from Illustrator-exported SVG files:
   - one centerline signature path
   - one stroke-only block-text outline with closed contours that draws
     sequentially, not in parallel
   - one multi-subpath same-style path
   - unsupported filled-outline text export, with a clear diagnostic or
     documented behavior

### Phase 3: SVG Import Tooling

1. Add `pax-cli svg-import`.
2. Support validation/debug output, stdout generation, and ejection with `--out`.
3. Keep command output deterministic so it can support golden tests.
4. Document that ejected Pax is editable source and no longer tracks the SVG
   automatically.

## Future Extensions

- `PathCubic` child component, if hand-authored hierarchical cubic paths become
  common.
- `draw_offset` for marching/chasing effects without forcing authors to compute
  modulo arithmetic themselves.
- `draw_wrap` for closed paths where `draw_start > draw_end` should wrap around
  the end of the contour.
- General dashed strokes, likely on `Stroke`, if authors need dash patterns
  independent of animation.
- Reuse `UnitValue` for other authoring surfaces that mean "a value in normalized
  unit space," such as gradient stop positions or material scalar factors.
- A mask-based `DrawReveal` wrapper for arbitrary vector descendants and filled
  illustration reveal.
- Shared path measurement utilities for PAX-943 "animate along path".
- Runtime SVG import for user-provided or network-provided SVG data, once a
  concrete app requires it.

## Open Questions

- Should first-pass `draw_start > draw_end` draw nothing or wrap?
  Recommendation: draw nothing first; wrapping can be explicit later.
- Should `Line` get the same properties immediately?
  Recommendation: not in the first pass. `Line` can be represented as a `Path`,
  and duplicating the API should wait until we see whether this belongs on every
  stroke-bearing primitive.
- Should the feature expose absolute length units in addition to normalized unit
  values?
  Recommendation: defer. `UnitValue` values compose best with timelines and
  responsive path bounds.
- How much SVG should the import phase support?
  Recommendation: start with path data, viewBox normalization, simple style
  extraction, transform flattening, and compatible-path merging needed for
  Illustrator centerline paths and stroke-only outline tracing.
  Defer full style inheritance, text, filters, clipping, masks, and arbitrary
  SVG rendering.
