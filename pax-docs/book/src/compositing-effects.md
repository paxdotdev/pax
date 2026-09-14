<a id="compositing--effects"></a>

# Compositing and Effects
<!-- summary: Compose rendered and native content with clipping, geometric and alpha masks, inherited opacity, and platform-specific effects. -->
<!-- tags: compositing, masking, clipping, opacity, native, effects -->

A note card might combine a photograph, a vector border, editable text,
and a native button. Compositing brings those pieces into one scene: what
appears in front, what remains visible through a shape, and how transparency
changes the result.

This chapter builds on [Layout](layout-responsiveness.md) and
[Drawing](drawing-styling.md). It covers the boundaries around content;
[Animation](animation-motion.md) explains how to move those boundaries and
change their properties over time.

## Choose the boundary

Earlier siblings in a Pax template appear in front of later siblings. Put
the label before its surface and the surface before its shadow. Nesting
gives a subtree a shared coordinate system, so moving or rotating its
container carries its descendants with it. See
[element order](template-language.md#element-order) and
[transforms](layout-responsiveness.md#transforms-and-origins) for those foundations.

Choose a container based on what should happen at its edge:

| Container | Use it for |
| --- | --- |
| `Group` | A shared transform and layout space, with content free to overflow |
| `Frame` | A rectangular or rounded-rectangular clipping boundary |
| `Mask` | A boundary supplied by another subtree's geometry or painted alpha |

These containers do not paint a background by themselves in an ordinary
scene. Add a Rectangle or another drawing primitive when you want a
surface. A rounded Rectangle changes its own silhouette; it does not clip
text, images, or other siblings to that silhouette.

## Clip with a Frame

A Frame clips its descendants to its bounds by default. `corner_radius`
rounds that boundary, using a number of pixels:

```pax
<Frame x=24px y=24px width=280px height=160px corner_radius=24>
    <Text x=16px y=16px width=248px height=32px text="A view of the coast"
        style={font: "Arial", font_size: 18px, fill: rgb(36, 54, 47)} />
    <Ellipse x=196px y=80px width=140px height=140px
        fill=rgb(72, 124, 112) />
    <Rectangle width=100% height=100% fill=rgb(237, 241, 226) />
</Frame>
```

The ellipse extends beyond the Frame's right and bottom edges. Only the
part inside the rounded boundary is visible. The Text participates in the
same clipped composition even though it uses a native text surface.

The radius is clamped to fit the smaller dimension. It is a single radius
for the clipping boundary; the four-corner list accepted by Rectangle is
a different property contract. For ordinary grouping without a crop, use
Group. [Layout](layout-responsiveness.md#choose-a-container) covers autosizing
and choosing the Frame's dimensions.

Clipping changes visibility, not the child's layout size or application
state. A clipped child remains mounted. Nested Frames further restrict the
visible area; a child's transform does not free it from an ancestor's clip.
Leave room inside that boundary for strokes and animated overshoot, or
place the decoration outside the clipped subtree.

Image fitting and clipping are separate choices. An Image handles its own
rectangular fit; a surrounding Frame can add rounded corners. See
[image fitting](text-fonts-images.md#fit-an-image-into-its-bounds).

## Shape content with a Mask

A Mask takes **exactly two direct children**:

1. The content to show.
2. The subtree that supplies the mask.

The `alpha` property selects how that second subtree is used:

| Mode | Source behavior | Content support |
| --- | --- | --- |
| `alpha=false` (default) | Geometric coverage gives a hard clipping boundary | Rendered and native content |
| `alpha=true` | Painted alpha controls how much content remains visible at each point | GPU-rendered canvas content |

Use a Group when either side contains several elements. Here, the first
Group contains a surface, native text, and a native button. The second
child is an ellipse that selects the visible area:

```pax
<Mask x=24px y=24px width=280px height=180px>
    <Group width=100% height=100%>
        <Button x=70px y=84px width=140px height=36px label="Open note" />
        <Text x=60px y=44px width=160px height=28px text="Moss and rain"
            style={font: "Arial", font_size: 18px, fill: rgb(36, 54, 47)} />
        <Rectangle width=100% height=100% fill=rgb(237, 241, 226) />
    </Group>
    <Ellipse x=20px y=10px width=240px height=160px fill=BLACK />
</Mask>
```

The ellipse itself is not painted as a black foreground object. Pax uses
its transformed coverage path to clip the first child's descendants,
including native surfaces. Black is simply a convenient fill for the
source shape.

Both sides use the Mask's coordinate space. Percentage dimensions resolve
against that space; positions, rotations, and nested transforms carry
through to the resulting coverage. Bind a source shape's position or size
to a property to move the visible window, or animate those values with a
[timeline](animation-motion.md#timelines). Moving the source changes the
window without moving the content underneath it.

### Use a path or a subtree

For a custom silhouette, replace the second child with a filled, closed
Path. This one cuts the corners off a rectangular label:

```pax
<Path width=100% height=100% fill=BLACK
    elements={[
        PathElement::Point(12%, 0%),
        PathElement::Line, PathElement::Point(88%, 0%),
        PathElement::Line, PathElement::Point(100%, 20%),
        PathElement::Line, PathElement::Point(100%, 80%),
        PathElement::Line, PathElement::Point(88%, 100%),
        PathElement::Line, PathElement::Point(12%, 100%),
        PathElement::Line, PathElement::Point(0%, 80%),
        PathElement::Line, PathElement::Point(0%, 20%),
        PathElement::Close
    ]} />
```

In the default geometry mode, Mask collects coverage paths from descendants of its second child.
A source Group can therefore contain several shapes. Keep the source
tree small and purposeful: only geometry with a coverage-path implementation
contributes. A Group by itself contributes no shape, and native text does
not turn into letter-shaped clipping geometry.

The source paths are combined for clipping; this is not a general boolean
geometry editor. For holes, overlapping contours, or self-intersections,
verify the result on the target renderer instead of assuming a particular
union or subtraction rule. Drawing owns
[path commands and SVG conversion](drawing-styling.md#paths-and-svg).

### Geometry and transparency

With `alpha=false`, the mask uses geometric coverage rather than painted
transparency. An Image contributes its rectangular bounds, not the silhouette
of its transparent pixels. Use `alpha=true` for soft coverage authored with
vector fills, strokes, and gradients, as shown below.

Use explicitly filled shapes for predictable source coverage. Shape APIs
decide which geometry contributes: for example, a Path with no visible fill
or stroke may have no coverage at all. If the source produces no coverage
path, geometry mode applies no clip; it does not hide
everything. Use conditional content when the intended state is “show nothing.”

A nested Mask can further constrain already-masked content. In geometry
mode, the source's paint appearance does not determine coverage. In either
mode, source-side Frame and Mask clips are not applied to the source paints;
keep the source tree focused on the shapes that define the reveal.

### Painted alpha masks

Set `alpha=true` to reveal content according to the source's painted alpha.
Opaque paint reveals fully, transparent paint hides, and partial alpha gives
partial visibility. The source's RGB color is irrelevant: opaque black and
opaque white both reveal fully. This is alpha masking, not luminance masking.

Here a gradient reveals a green surface through the middle and fades it
away at both ends:

```pax
<Mask x=24px y=24px width=280px height=120px alpha=true>
    <Rectangle width=100% height=100% fill=rgb(56, 100, 78) />
    <Rectangle width=100% height=100%
        fill=@gradient {
            linear: { start: [0%, 50%] end: [100%, 50%] }
            0%: rgba(0, 0, 0, 0)
            25%: rgba(0, 0, 0, 255)
            75%: rgba(0, 0, 0, 255)
            100%: rgba(0, 0, 0, 0)
        } />
</Mask>
```

Rectangle, Ellipse, and Path fills and strokes can supply alpha, including
solid alpha and linear or radial gradient stops. Source transforms and
opacity apply too. Use up to eight ordered gradient stops. A Group or
repeated subtree can combine supported source shapes; overlapping paints
combine with source-over alpha, so two half-opacity shapes give 75%
coverage where they overlap.

Nested alpha masks on the content multiply their coverage, while ordinary
geometric clips further restrict the result. An empty or fully transparent
alpha source hides all content. Alpha masking modulates individual canvas
draws; it does not flatten the content into an isolated group before applying
transparency.

The current alpha path requires the GPU renderer. Native text and controls
are not supported as content inside an alpha mask, and Piet does not support
alpha masking. Images, text, native elements, and source-side clipping are
not supported alpha sources. For a mixed native/rendered composition, keep
the default geometry mask; for a soft visual reveal, use supported vector
source paint and keep interactive hit targets separate. Alpha coverage does
not change hit testing.

<div class="docs-example-placeholder">
<p><strong>Interactive example planned:</strong> compare a hard geometry mask with a gradient-alpha reveal, showing the content, source, and result side by side.</p>
<!-- Production brief:
- Expose geometry controls and a source-outline toggle outside the actual Mask.
  Keep native text and a working button in the content; compare Frame and Mask.
- Use separate GPU canvas content for the alpha example, with an explicit
  backend label and no native text or controls inside the alpha-masked content.
- Three treatments: a Field notes specimen window; a paper viewfinder over a
  landscape; a ticket-shaped crop. Prefer the specimen window and a static path.
- Show the two direct children in source tabs. Include a still diagram and
  keyboard controls; do not rely on autoplay or on color alone. -->
</div>

## Opacity through a subtree

`opacity` accepts a normalized value such as `0.5`, or a percentage such as
`50%`. Each node inherits its render parent's opacity and multiplies it by
its own value. Paint alpha is another multiplier:

```pax
<Group x=24px y=24px width=280px height=120px opacity=50%>
    <Rectangle x=0px y=0px width=160px height=100px
        fill=rgb(56, 100, 78) />
    <Rectangle x=100px y=20px width=160px height=100px
        fill=rgb(56, 100, 78) opacity=50% />
</Group>
```

The first Rectangle paints at 50% opacity. The second inherits that 50%
and multiplies it by another 50%, so it paints at 25%. Giving the second
Rectangle `opacity=1` would preserve the inherited 50%; it would not undo
the parent's attenuation. Drawing explains
[paint alpha](drawing-styling.md#color-and-transparency), including `rgba` units.

This is per-descendant opacity. Group, Frame, and Mask do not provide a
general “render the subtree to one image, then fade that image” isolation
operation. Overlapping translucent descendants can build up opacity where
they overlap. Adding another Group does not remove that buildup.

Choose the property that matches the visual intent. To soften a card's
background while keeping its label crisp, lower the background fill's
alpha. To fade the card's individual pieces together, change a shared
ancestor's opacity and inspect their overlaps. Two overlapping copies at
50% each do not produce one fully opaque copy; avoid complementary
crossfades when a silhouette must stay solid throughout the handoff.

Opacity does not unmount a component, disable a control, or manage keyboard
focus. Keep those state and interaction decisions explicit. Likewise, a
visual mask is not an accessibility description or a promise of pixel-exact
hit testing for every control. See
[Accessibility and Native Controls](accessibility-native-controls.md#focus-and-keyboard-interaction).

## Native compositing

Pax draws vector primitives and Image content through its renderer. Text,
NativeImage, and form controls use native surfaces supplied by the browser
or operating system. Those surfaces share the Pax scene's transforms and
ordering, even though they are not all painted into the same canvas.

To make rendered content appear above a native control, Pax computes its
coverage and masks the corresponding portion of the native surface. This
is often called **punch-through** in the implementation. You normally
express the desired order through the template:

```pax
<Group x=24px y=24px width=280px height=120px>
    <Ellipse x=180px y=0px width=80px height=80px fill=rgb(207, 120, 79) />
    <Button x=24px y=24px width=220px height=44px label="Partly covered" />
    <Rectangle width=100% height=100% fill=rgb(237, 241, 226) />
</Group>
```

The ellipse is earlier in the template, so its overlapping area appears in
front of the Button. The later Rectangle stays behind both. Moving the
ellipse updates that relationship; wrapping the composition in a Mask
adds another visible boundary.

<div class="docs-media-placeholder">
<p><strong>Diagram planned:</strong> show the template order, rendered surface, native surface, and coverage mask for the overlapping button and ellipse.</p>
<!-- Production brief:
- Explain the final image first, then separate its contributing surfaces.
- Three treatments: exploded paper layers; a four-panel before/after; a simple
  annotated cutaway. Prefer the cutaway, with labels that do not imply one
  offscreen texture or native layer for every Pax component.
- Include a scroller-island inset only after the root-surface explanation. -->
</div>

### Coverage has limits

Native punch-through uses coverage geometry and an opacity estimate;
it does not sample every final rendered pixel. This matters for subtle
cross-surface blends:

- A gradient with changing alpha has one estimated coverage opacity for
  the native mask, rather than a separate alpha value at each pixel.
- A rendered Image's coverage is rectangular, including transparent pixels
  in the source image.
- A partially revealed Path can retain conservative coverage for its full
  stroke so the runtime does not rebuild native occlusion geometry on every
  animation sample.

These cases can differ from placing both pieces in a single paint surface.
Keep important text and controls clear of such overlaps, and verify an
intentional cross-surface effect on the target where it will ship. The
`occlusion` and `neon-opacity` examples are useful places to explore the
current behavior.

### Scroller islands and modal underlays

A native Scroller can host its own rendered-content surface, often called
an **island**. Its scrolling and clipping belong to that host. A translucent
Rectangle on the root canvas therefore cannot be assumed to tint every
native control and scroller island as if the whole interface were one image.

For a modal background that must both dim the scene and absorb pointer
input, use an EventBlocker with a translucent solid `background`. Place the
panel before it and the underlying content after it:

```pax
<Group width=100% height=100%>
    <Group x=50% y=50% width=280px height=120px>
        <Text x=20px y=20px width=240px height=32px text="A moment to pause"
            style={font: "Arial", font_size: 18px, fill: rgb(36, 54, 47)} />
        <Rectangle width=100% height=100% corner_radius=16
            fill=rgb(237, 241, 226) />
    </Group>
    <EventBlocker width=100% height=100% background=rgba(0, 0, 0, 128) />
    <Button x=24px y=24px width=160px height=44px label="Underlying action" />
    <Rectangle width=100% height=100% fill=rgb(248, 244, 234) />
</Group>
```

This demonstrates the surface ordering, not a complete modal component.
In an application, mount the panel and blocker conditionally and provide
dismissal and focus behavior. EventBlocker's background is transparent by
default. It is a native surface that absorbs pointer input; it does not
establish a keyboard focus trap. [Routing](routing.md) covers route-driven
panels and their lifecycle.

## Platform-specific effects

### Apple Liquid Glass

LiquidGlass provides an inherited effect scope for supported Apple native
surfaces. A Group inside it can supply a rounded glass surface, while
supported controls receive the native treatment:

```pax
<LiquidGlass x=24px y=24px width=280px height=160px
    spacing=12px variant="regular">
    <Group width=100% height=100% corner_radius=20>
        <Text x=20px y=20px width=240px height=32px text="Field notes"
            style={font: "Arial", font_size: 18px, fill: rgb(36, 54, 47)} />
        <Button x=20px y=76px width=240px height=44px label="Continue" />
    </Group>
</LiquidGlass>
```

`variant` supports `"regular"` and `"clear"`. `tint` supplies an optional
color; `spacing` controls grouping distance. `interactive=true` requests
the interactive treatment where supported. A nested
`<LiquidGlass enabled=false>` opts its descendants out of the inherited scope.

The Apple implementation uses native glass on macOS 26 and iOS/iPadOS 26
or later, with native visual-effect fallbacks on earlier supported OS
versions. The details differ by platform; for example, the current UIKit
implementation applies the interactive flag, while the macOS implementation
does not provide the same response. Web retains ordinary controls without
the Apple glass treatment. Give that fallback an intentional, readable
composition rather than depending on glass for contrast.

Use `examples/src/liquid-glass` to explore the native effect and nested
opt-outs on Apple targets. A successful web run checks the fallback, not
the Apple appearance. The [LiquidGlass API](api/pax-std/core/liquid_glass.md)
lists the scope's properties.

### Lighting and other effects

Light-reactive vector materials and LightFrame are taught in
[Drawing](drawing-styling.md#lighting-and-materials). Their GPU lighting
changes the drawn material; it does not automatically blur or refract all
the native content behind it. Piet renders those materials unlit.

A general-purpose Gaussian blur wrapper, configurable blend modes, and
portable backdrop materials are not part of the current public toolkit.
You can still build depth with offset shapes, gradients, transparency,
and motion. Keep authored shadows and glows distinct from a sampled
backdrop effect when explaining what an example demonstrates.

## Check the finished composition

Start with the smallest composition that shows the intended relationship.
Check it at rest, after resizing, during scrolling, and at intermediate
animation values. Include a native control in the test if the finished
interface mixes native and rendered content; an all-vector mockup does
not exercise that boundary.

When something disappears, inspect source order and ancestor clips first,
then Mask child order and source coverage, followed by inherited opacity.
If the issue appears only while crossing a native surface or Scroller,
compare the surface arrangement as well as the element's local geometry.

From the repository root, run a canonical testbed:

```sh
pax-cli run --path examples/src/occlusion --target web
```

`neon-opacity` adds animated nested transparency and mixed controls.
Verify the target and renderer you intend to ship: web, macOS, iOS, and
iPadOS share the authoring model, but their native surfaces and effect
implementations have the qualifications described here.

## Read more

Continue with [Scrolling and Viewports](scrolling-viewports.md) for content
coordinates and scroll-driven composition. Revisit
[Layout](layout-responsiveness.md) for transforms and bounds,
[Drawing](drawing-styling.md) for paint and geometry, and
[Animation](animation-motion.md) for changing those values over time.
The [Frame](api/pax-std/core/frame.md), [Mask](api/pax-std/core/mask.md), and
[EventBlocker](api/pax-std/core/event_blocker.md) references list their public APIs.
