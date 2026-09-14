<a id="drawing--styling"></a>

# Drawing and Styling
<!-- summary: Shapes, color, strokes, gradients, reusable visual settings, paths, SVG, and lighting. -->
<!-- tags: drawing, styling, gradients, paths, svg, materials -->

A few drawing decisions can give a small interface a recognizable character:
a warm surface, a fine outline, a generous corner, a line that guides the eye.
Pax's vector elements let you make those decisions in the same template that
positions your text and controls.

This chapter builds on [Layout](layout-responsiveness.md) and
[PAXEL](data-binding-expressions.md). The examples can go inside a component's
template, with `use pax_kit::*;` in its Rust file. Start with ordinary shapes
and paint; the later sections introduce custom paths, SVG, and lighting.

<a id="shapes-and-gradients"></a>

## Compose a surface

Here is a small surface for the Field notes content from the previous chapter:

```pax
<Group x=24px y=24px width=280px height=156px>
    <Text x=24px y=24px width=232px height=36px text="Field notes"
        style={font: "Arial", font_size: 24px, fill: rgb(36, 54, 47)} />
    <Line x=24px y=80px width=232px height=1px
        x1=0px y1=0px x2=100% y2=0px
        stroke={color: rgb(152, 168, 146), width: 1px} />
    <Ellipse x=24px y=108px width=12px height=12px fill=rgb(67, 112, 86) />
    <Rectangle width=100% height=100% corner_radius=18
        fill=rgb(237, 241, 226)
        stroke={color: rgb(178, 192, 168), width: 1px} />
</Group>
```

The rectangle is last because it sits behind the other elements. Keep this
foreground-first order when adding a highlight or a background; see
[Element ordering](template-language.md#element-order).

Rectangle fills its bounds; Ellipse fits an oval inside them. Equal width and
height give Ellipse a circle. Line connects two local endpoints, while Path
describes a longer sequence of segments and curves. They share layout
properties with other elements, so percentages, transforms, and reactive
bindings apply to drawings too.

A Group organizes these elements but does not paint a background of its own.
Likewise, rounding a Rectangle changes that rectangle's geometry; it does not
clip its siblings. Use a [Frame](layout-responsiveness.md#choose-a-container) when content
needs to stay inside a rounded boundary.

## Color and transparency

Use `rgb` for an opaque color and `rgba` when its paint should be translucent.
RGB channels and an integer alpha use the range 0–255. Percent channels are
also accepted. These two rectangles use equivalent half-opacity paint:

```pax
<Rectangle x=24px y=24px width=120px height=72px
    fill=rgba(56, 120, 91, 50%) />
<Rectangle x=160px y=24px width=120px height=72px
    fill=rgb(56, 120, 91) opacity=50% />
```

For an integer alpha, `255` is fully opaque and `128` is approximately half.
Use `50%` when you mean an exact half: a decimal `0.5` in `rgba` is not a
normalized alpha value. Element `opacity`, by contrast, accepts `0.5` or
`50%`. Paint alpha and element opacity multiply, so applying both halves
leaves one-quarter opacity for that paint.

`hsl(150deg, 30%, 45%)` describes hue, saturation, and lightness;
`hsla(150deg, 30%, 45%, 50%)` adds alpha. Named colors such as `WHITE`, `BLACK`,
`BLUE`, and `TEAL` are convenient palette values. The colored names use Pax's
palette—`RED`, for example, is not a synonym for `rgb(255, 0, 0)`. Use explicit
channels when matching a design precisely. `TRANSPARENT` provides clear paint.

For a vector, `fill` paints the interior and `stroke` paints its outline.
An omitted fill uses the default SLATE color; an omitted stroke has zero
width. Set `fill=TRANSPARENT` when you want only an outline.

Opacity on a container also affects its descendants. The behavior of
overlapping children and native elements belongs to
[Compositing](compositing-effects.md); do not assume a group fades as a
single flattened image. Text's `style.fill` has its own native-rendering
limits, covered in [Text and fonts](text-fonts-images.md#display-text).

## Outlines and corners

### Strokes

A stroke combines a color, pixel width, cap, and join. It is centered on the
path, so a 4px outline extends about 2px to either side of its centerline.
Leave room for that extension near a clipping edge.

```pax
<Line x=24px y=24px width=240px height=40px
    x1=8px y1=20px x2={100% - 8px} y2=20px
    stroke={
        color: rgb(56, 120, 91)
        width: 8px
        cap: StrokeCap::Round
    }
/>
```

`StrokeCap::Butt` ends at the endpoint, `Round` adds a semicircle, and `Square`
extends a squared end by half the stroke width. Butt is the default. Closed
contours have no exposed endpoints, so cap selection is useful for Line and
open Path contours.

For joined segments, choose `StrokeJoin::Miter`, `Round`, or `Bevel`. Miter is
the default and extends edges toward a point; Round softens the turn; Bevel
cuts the corner across. A stroke's paint is a `Color`, not a gradient `Fill`.
Use pixel widths such as `2px`; percentage stroke widths are not supported by
the current vector renderers.

### Corner radii

Rectangle accepts one to four corner values. They are unitless numbers
representing local pixel radii. A single number rounds every corner:

```pax
<Rectangle x=24px y=24px width=240px height=100px
    corner_radius=[24, 8, 24, 8] fill=rgb(220, 232, 211) />
```

The list expands clockwise from the top-left:

| Input | Top-left | Top-right | Bottom-right | Bottom-left |
| --- | --- | --- | --- | --- |
| `12` or `[12]` | 12 | 12 | 12 | 12 |
| `[12, 6]` | 12 | 6 | 12 | 6 |
| `[12, 6, 3]` | 12 | 6 | 3 | 6 |
| `[12, 6, 3, 1]` | 12 | 6 | 3 | 1 |

Use a named object when the corner names make an asymmetric shape clearer:
`corner_radius={top_left: 12, top_right: 6, bottom_right: 3, bottom_left: 1}`.
The explicit `CornerRadii { ... }` form is also available. Size, anchoring,
and transforms remain the responsibilities described in Layout.

## Gradients

A gradient varies the fill across a shape. `@gradient` lists the colors at
positions along it; the renderer blends between those stops:

```pax
<Rectangle x=24px y=24px width=280px height=140px corner_radius=18
    fill=@gradient {
        linear: {
            start: [0%, 0%]
            end: [100%, 100%]
        }
        0%: rgb(237, 241, 226)
        55%: rgb(173, 205, 173)
        100%: rgb(67, 112, 86)
    }
/>
```

Points are `[x, y]` pairs in the shape's local coordinate space. Here the
gradient runs from the top-left to the bottom-right. Percent coordinates
follow the shape's bounds as it resizes. Use at least two stops in ascending
order and write their positions as percentages; the Piet fallback requires
percentage stops even though the GPU renderer can also interpret pixels.

Omitting the `linear` block gives a left-to-right gradient, from `[0%, 0%]`
to `[100%, 0%]`:

```pax
<Rectangle x=24px y=24px width=280px height=80px
    fill=@gradient {
        0%: rgb(237, 241, 226)
        100%: rgb(67, 112, 86)
    }
/>
```

Put transparency in each stop's color, such as `rgba(255, 255, 255, 0)`.
There is no separate stop-opacity field. A translucent gradient over a
surface can supply a highlight while preserving the underlying color.

Radial gradients are also available through a `radial` block with `start`,
`end`, and `radius`. Their geometry currently differs between the GPU and
Piet renderers: GPU uses the start-to-end vector scaled by radius; Piet uses
origin/center points and a radius. In particular, equal start/end points
collapse the GPU gradient's axis. Treat radial fills as backend-sensitive
and verify their appearance on your shipping targets; the linear examples
above are the starting point for this chapter.

<div class="docs-example-placeholder">
<p><strong>Interactive example planned:</strong> tune a small surface's palette, outline, corners, and gradient while viewing its source.</p>
<!-- Production brief:
- Show equivalent alpha/opacity, centered stroke extents, corner-list mapping,
  gradient direction, and narrow/wide bounds. Keep the geometry small and legible.
- Three treatments: a Field notes cover; a transit-ticket designer; a set of
  botanical specimen labels. Prefer the cover to continue the learning thread.
- Use canonical source tabs and a static fallback. No new gallery ownership. -->
</div>

## Reusable visual settings

Choose a small vocabulary for your interface: perhaps a paper surface, a
quiet outline, an accent color, and two corner sizes. Classes can name those
roles so that repeated elements stay consistent:

```pax
<Rectangle x=24px y=24px width=120px height=80px class="paper" />
<Rectangle x=160px y=24px width=120px height=80px class="paper"
    fill=rgb(219, 233, 212) />

@settings {
    .paper {
        fill: rgb(237, 241, 226)
        stroke: {color: rgb(178, 192, 168), width: 1px}
        corner_radius: 16
    }
}
```

Both surfaces share an outline and corner shape; the second supplies its own
fill inline. Class names describe a role here, which makes them useful when
the palette changes. Keep typography roles in the same visual system, using
TextStyle on the Text elements that need them.

When several components need those choices, use a theme component through
[`ImportSettings`](template-language.md#imported-settings). Templates owns
the provider setup, scope, and override order. A child component imports the
settings needed by its own template; placing a theme above it does not
automatically style its internals.

The canonical `examples/src/runtime-settings-themes` example separates color,
typography, and corners into providers and switches them with ordinary
reactive state. It is a useful next step when one shared class has grown into
a theme. Keep event handlers responsible for state changes and let bindings
select the visual values, as described in [PAXEL](data-binding-expressions.md).

## Paths and SVG

### Describe a path

Path's `elements` property holds commands and points. Start with a Point,
then add a segment command followed by its endpoint:

```pax
<Path x=24px y=24px width=280px height=120px fill=TRANSPARENT
    stroke={color: rgb(56, 120, 91), width: 4px, cap: StrokeCap::Round}
    elements=[
        PathElement::Point(0%, 75%),
        PathElement::Cubic(25%, 0%, 75%, 100%),
        PathElement::Point(100%, 25%)
    ]
/>
```

The Cubic supplies two control points, which pull the curve toward them;
the following Point supplies its destination. Coordinates belong to the
Path's own bounds. Percentages reshape the curve with those bounds; pixels
give fixed local distances.

| Command | Meaning |
| --- | --- |
| `Point(x, y)` by itself | Move to a point and begin a new contour |
| `Line`, then `Point(x, y)` | Draw a straight segment |
| `Quadratic(cx, cy)`, then `Point(x, y)` | Draw a curve with one control point |
| `Cubic(c1x, c1y, c2x, c2y)`, then `Point(x, y)` | Draw a curve with two control points |
| `Close` | Connect back to the contour's starting point |

These entries all use the `PathElement::` prefix in a template. Close a
contour deliberately when drawing a filled shape:

```pax
<Path x=24px y=24px width=160px height=100px
    fill=rgb(220, 232, 211)
    stroke={color: rgb(56, 120, 91), width: 3px, join: StrokeJoin::Round}
    elements=[
        PathElement::Point(50%, 8%),
        PathElement::Line, PathElement::Point(92%, 92%),
        PathElement::Line, PathElement::Point(8%, 92%),
        PathElement::Close
    ]
/>
```

For data-driven drawings, keep a `Property<Vec<PathElement>>` in Rust and
bind it to `elements`. Updating the property changes the drawing through
the usual reactive loop. Rust helpers `Path::start`, `Path::line_to`, and
`Path::curve_to` can assemble a command list; the last makes a quadratic
curve. See [Properties](state-properties.md#collections-and-value-snapshots)
for collection updates.

Path geometry may extend outside its layout bounds. Use Frame or Mask when
you want clipping, rather than relying on width and height to crop it.
`smoothing=PathSmoothing::Light` or `Strong` can soften polyline runs;
the default `None` preserves the authored geometry. Smoothing changes the
shape, so inspect corners and lettering after enabling it.

<div class="docs-media-placeholder">
<p><strong>Diagram planned:</strong> annotate the cubic curve with its two endpoints, two control points, and local percentage bounds.</p>
<!-- Production brief:
- Show which Point ends the Cubic and how widening the bounds reshapes it.
- Three treatments: a drafting-board diagram; a string-and-pins illustration;
  a graph-editor screenshot. Prefer the drafting board with literal labels. -->
</div>

### Reveal a stroke

`draw_start` and `draw_end` select a range along the total path length. This
line shows its first half:

```pax
<Path x=24px y=24px width=280px height=40px fill=TRANSPARENT
    stroke={color: rgb(56, 120, 91), width: 6px, cap: StrokeCap::Round}
    draw_start=0% draw_end=50%
    elements=[
        PathElement::Point(0%, 50%),
        PathElement::Line, PathElement::Point(100%, 50%)
    ]
/>
```

The default range is 0 to 1, showing the full stroke. `0.5` and `50%` mean the
same progress; values are clamped to this range. An empty or reversed range
shows no stroke. The fill remains intact: these properties reveal the
outline, so use a transparent fill for a pen-like drawing effect.

On the GPU renderer, the revealed range cuts across the existing stroke;
round caps at the original endpoints do not add a rounded tip at that cut.

Bind the range to application state or animate it with a timeline. The
`path-drawing` example combines authored curves and imported lettering;
[Animation and Motion](animation-motion.md) owns the timing and playback
model. `Handwriter` is a higher-level component that turns text into paths
using bundled stroke fonts, with the same draw-range controls. See its
[API reference](api/pax-std/drawing/handwriter.md) for font and text options.

### Bring in an SVG

SVG import converts supported vector artwork into Pax Paths at build time.
First validate the source from your project directory:

```sh
pax-cli svg-import assets/signature.svg
```

For a concrete source to try, copy
`examples/src/path-drawing/assets/svg/signature-strokes.svg` to that location.
The command reports its viewBox, generated path count, and warnings. Review
those warnings and compare the result visually before adopting the artwork.

The importer requires a valid viewBox and supports paths and polygons, line
and Bézier commands, transforms, solid fills, and strokes. It is a bounded
subset of SVG: convert shapes and text to paths before importing; convert
arc commands to curves. Gradient paints are rejected. Filters, masks, and
other unsupported elements are not reproduced. Complex SVG styling and
compositing need a separate fidelity check.

To keep the SVG as your source, declare a component in Rust. This example
can live in `src/signature.rs`:

```rust
use pax_kit::*;

#[pax]
#[custom(Default)]
#[svg("assets/signature.svg")]
pub struct Signature {
    pub draw_start: Property<UnitValue>,
    pub draw_end: Property<UnitValue>,
}

impl Default for Signature {
    fn default() -> Self {
        Self {
            draw_start: Property::new(UnitValue::Unitless(0.0.into())),
            draw_end: Property::new(UnitValue::Unitless(1.0.into())),
        }
    }
}
```

Declare `pub mod signature;` and `use signature::Signature;` in `src/lib.rs`,
then place the component in the template:

```pax
<Signature x=24px y=24px width=320px height=80px />
```

The example preserves the source's 4:1 aspect ratio. Imported coordinates
scale into the component's allocated bounds; use a matching ratio when you
want the original shape proportions. Emitted stroke widths remain pixel
values, so inspect their weight at the size you intend to display. The
draw-range properties let the generated strokes participate in the same
reveal workflow as a hand-authored Path.

To edit the generated Pax instead, eject a component:

```sh
pax-cli svg-import assets/signature.svg --component Signature --out src/signature
```

This writes `src/signature.rs` and `src/signature.pax`; use that pair **instead
of** the `#[svg]` declaration above. Existing files are protected unless you
explicitly pass `--force`. After ejection, the Pax file is editable source:
changes to the original SVG do not automatically update it. To inspect the
generated template without writing files, use `--stdout`.

## Lighting and materials

Vector materials can respond to authored lights on the GPU renderer. A
LightSource describes illumination; a Material describes how a surface
responds. LightFrame keeps a local light from spilling into neighboring
component instances:

```pax
<LightFrame x=24px y=24px width=280px height=160px>
    <Rectangle width=100% height=100% corner_radius=20
        fill=rgb(67, 112, 86) material={Material::glossy(0.6)} />
    <LightSource x=64px y=40px width=1px height=1px
        z=90px radius=240px intensity=1.5 color=rgb(255, 231, 192) />
</LightFrame>
```

LightSource is a non-rendering resource; it does not draw a visible lamp.
Its point-light position uses its local center plus the authored depth.
Use pixel values for `radius` and `z`. Depth here affects lighting, not the
element order established by the template. Directional lights are also
available through `shape=LightShape::Directional` and a `direction` vector.

The default vector material is matte. `Material::glossy(...)` and
`Material::metallic(...)` adjust the response; `Material::unlit()` preserves
the authored paint independently of lights. `Material::emissive(...)` adds
color to its own surface; it does not turn the shape into a light source
for its neighbors. The [material reference](api/pax-runtime-api/drawing.md)
describes explicit response coefficients.

LightFrame containment is one-way: outside lights can enter, while lights
inside cannot affect parents or sibling frames. Canvas-layer reachability
still applies, and `AmbientLight` is layer-wide rather than confined by
LightFrame. A primitive without eligible direct lights or authored ambient
keeps its unlit appearance.

This feature is backend-dependent. GPU/WGPU rendering supports the material
response, with a current limit of eight active direct lights in each canvas
layer's lighting set, shared across its LightFrames. The Piet fallback draws
the ordinary fills and strokes without
that response. Native Text, native controls, and bitmap Images are not lit
vector materials. Texture maps, cast shadows, and a 3D camera are not part
of this API. Keep information legible when lighting is absent.

The canonical `glow-buttons` example shows a pointer-following light scoped
to each button; `materials` explores surface response. They are useful
references after the small static scene above.

<div class="docs-example-placeholder">
<p><strong>Interactive example planned:</strong> move a light over two adjacent surfaces and compare glossy, matte, and unlit paint.</p>
<!-- Production brief:
- Reuse glow-buttons/materials source. Show sibling isolation, ancestor light
  entry, an unlit label, and a plainly labeled fallback without GPU lighting.
- Three treatments: mineral samples; two illuminated instrument buttons;
  glazed ceramic tiles. Prefer two buttons for a compact scoping demonstration.
- Keep source version-aligned and include a static comparison. -->
</div>

## Read more

Continue with [Native Controls](accessibility-native-controls.md) to add familiar
inputs, [Animation and Motion](animation-motion.md) to change these values
over time, and [Compositing](compositing-effects.md) to combine clipping,
masking, and native content.

The API references cover [Rectangle and corners](api/pax-std/drawing/rectangle.md),
[Ellipse](api/pax-std/drawing/ellipse.md), [Line](api/pax-std/drawing/line.md),
[Path](api/pax-std/drawing/path.md), [colors](api/pax-runtime-api/color.md),
and [light resources](api/pax-std/drawing/lighting.md).
