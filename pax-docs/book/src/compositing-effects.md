<a id="compositing--effects"></a>

# Compositing and Effects
<!-- summary: Compose rendered and native content with clipping, geometric and alpha masks, subtree opacity, and platform-specific effects. -->
<!-- tags: compositing, masking, clipping, opacity, native, effects -->

A note card might combine a photograph, a vector border, editable text,
and a native button. Compositing brings those pieces into one scene: what
appears in front, what remains visible through a shape, and how transparency
changes the result.

This chapter builds on [Layout](layout-responsiveness.md) and
[Drawing](drawing-styling.md). It covers the boundaries around content;
[Animation](animation-motion.md) explains how to move those boundaries and
change their properties over time.

Neon Opacity's **Afterimage Observatory** combines moving vector layers,
transparency, and masked native controls. Adjust the **BEAM** slider in the
Console to change the shared opacity (initially 75%), then try the text field or
**ENGAGE** button while the scene moves. Watch how overlapping surfaces and
their controls fade together. **Open standalone** gives the composition more
room to explore.

The masks in this example use geometric coverage; the
[painted alpha mask](#painted-alpha-masks) section below describes a separate
mode. Its motion is driven by frame-based Rust updates.

<pax-example
  path="neon-opacity"
  title="Neon Opacity"
  height="900"
  files="src/lib.pax,src/lib.rs">
</pax-example>

On iOS and iPadOS, the chassis applies native scene updates and presents Metal
surfaces in the same Core Animation transaction. Native text and Scroller
occlusion masks are also resolved and applied synchronously in that update,
so a new popup does not wait on separate mask callbacks to cover underlying
rows. Unchanged masks reuse their applied image; changed masks use a raster
cache or are generated before presentation. Cache misses therefore contribute
to frame time. Image loading remains asynchronous; supply a placeholder if
an image must be visible immediately.

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

Add `feather=2.0` to the Mask to soften its painted coverage. `feather` is the
Gaussian standard deviation in logical pixels, rather than a percentage or a
Pax length literal. It defaults to zero; negative values are treated as zero.
It affects alpha masks only and does not blur the revealed content itself.

Living Quilt uses `alpha=true feather=2.0` for its moving color waves. See the
[live Living Quilt and its source](what-is-pax.md#try-it-living-quilt) for the
complete two-child composition: a colored quilt followed by a Group of repeated
animated paths that supplies the coverage.

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

## Opacity through a subtree

`opacity` accepts a normalized value such as `0.5`, or a percentage such as
`50%`. On WGPU and the browser's Piet/Canvas2D renderer, it fades the composed
canvas content in a subtree. There is
no separate isolation setting. For example, these overlapping rectangles
form one silhouette with uniform 50% opacity:

```pax
<Group x=24px y=24px width=280px height=120px opacity=50%>
    <Rectangle x=0px y=0px width=160px height=100px
        fill=rgb(56, 100, 78) />
    <Rectangle x=100px y=20px width=160px height=100px
        fill=rgb(56, 100, 78) />
</Group>
```

The rectangles compose at their own paint alpha first; the Group then fades
that result once. Nested opacity works the same way: a child with `opacity=50%`
fades inside its parent, and a parent at 50% fades the assembled result again.
Setting a child to `opacity=1` cannot cancel an ancestor's fade. A Group does
not add clipping; use a Frame or Mask to restrict overflow.

Paint alpha remains useful for a different purpose. Lower a background's fill
alpha to let content behind it show through while its label stays crisp. Set
the container's opacity to fade its assembled canvas artwork. See
[paint alpha](drawing-styling.md#color-and-transparency), including `rgba` units.
Independent sibling fades still overlap using source-over; complementary
opacities do not keep a crossfading silhouette fully opaque.

**Current backend limits:** both renderers compose each canvas portion
separately. Live native text and controls, and content in separate native
Scroller surfaces, fade independently. Their overlap can therefore differ
from an exact image of the whole mixed subtree. The native coverage masks
below remain approximate. The focused `examples/src/group-compositing` fixture
has matching overlap probes on web WGPU, native macOS, iPhone/iPad simulators
and a physical iPad. Its interrupted exit preserves edited native text on
web, macOS and that iPad. These checks do not establish full accessibility
coverage or performance for larger applications. Piet's browser fixture also
matches the overlap reference and preserves native input during reversal.
There is no author-selectable legacy mode.

WGPU retains visible group content across opacity changes. Source image,
paint, transform, clip and lighting changes invalidate that content; outer
translation and native mask updates still perform work. Group surfaces add
memory and composition passes, so caching is not a guarantee of faster frames.
Piet replays dirty canvas content and reuses temporary canvases for composition;
it does not retain composed group pixels between dirty frames. Fully opaque
scopes bypass the temporary canvas, and fully transparent scopes skip painting.
Each simultaneously nested translucent scope can require another tile-sized
canvas. The browser still owns live native text and controls.

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

On Apple targets, changing coverage can require CPU rasterization of native
punch-through masks. These masks store one coverage byte per pixel, and unchanged
results are cached. A moving or fading overlay can invalidate that cache every
frame, so a large masked surface can still be expensive. Native and rendered
content are published synchronously to keep their coverage aligned. The browser
uses SVG/CSS masks; the Apple rasterization cost is not a measurement of that path.

### Coverage has limits

Native punch-through uses coverage geometry and an opacity estimate;
it does not sample every final rendered pixel. This matters for subtle
cross-surface blends:

In particular, a translucent canvas popup above a native Scroller does not
behave like an isolated composited surface above that Scroller. The underlying
canvas is attenuated again where the partially masked native scroll surface
covers it. During a fade, artwork can therefore appear stronger in row gutters
than over the rows themselves, even when the masks update synchronously.
Per-primitive opacity does not correct this cross-surface blending limitation.

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

On native iOS and iPadOS, the blocker retains UIKit touch ownership so the
content behind it cannot scroll. It forwards touch sequences into Pax's scene
hit test, allowing GPU-rendered panel controls above it and an `@click` handler
on the blocker itself to work. A single-finger tap activates once; drags,
cancelled touches, and multi-finger sequences do not become backdrop clicks.

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

In **Materials**, scroll through the collection of imagined elements. Scrolling
moves a light across surfaces with matte, glossy, metallic, emissive, and custom
materials. Compare how their highlights respond, then open the Rust source to
see the material parameters and scroll-driven light position. The lighting
requires the WGPU renderer; Piet shows the unlit fills and strokes.

<pax-example
  path="materials"
  title="Materials"
  height="760"
  files="src/lib.pax,src/lib.rs">
</pax-example>

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
then Mask child order and source coverage, followed by subtree opacity and the current native/canvas limits.
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
