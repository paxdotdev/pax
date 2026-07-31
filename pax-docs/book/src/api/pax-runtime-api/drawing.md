# drawing
<!-- summary: Vector drawing primitives: paths, fills, strokes, caps, and gradients. -->
<!-- tags: api, pax-runtime-api -->

Vector drawing primitives: paths, fills, strokes, caps, and gradients.

## Submodules
- [drawing::path_smoothing](drawing/path_smoothing.md)
- [drawing::path_trim](drawing/path_trim.md)
- [drawing::stroke_utils](drawing/stroke_utils.md)

## Structs
### `Depth`
Logical scene depth for lighting calculations.

`Depth` is expressed in logical pixels. Unlike [`Size`], it is not anchored
to a viewport or parent box, so percent and combined units are intentionally
rejected during value coercion.

#### Properties
##### `0`
Type: [`Numeric`](/api/pax-runtime-api/pax_value/numeric.md#numeric)

#### Implementations
##### `to_float`
<pre><code class="api-signature language-rust ignore">pub fn to_float(&amp;self) -&gt; f64</code></pre>

Returns the depth as a floating-point logical pixel value.

---

### `GradientStop`
A color stop for a gradient fill, defined by a position (% or px) and a color.

#### Properties
##### `position`
Type: [`Size`](/api/pax-runtime-api/layout.md#size)

Stop position, conventionally expressed as a percentage along the gradient.

##### `color`
Type: [`Color`](/api/pax-runtime-api/color.md#color)

Color at this stop.

#### Implementations
##### `get`
<pre><code class="api-signature language-rust ignore">pub fn get(color: <a href="/api/pax-runtime-api/color.md#color">Color</a>, position: <a href="/api/pax-runtime-api/layout.md#size">Size</a>) -&gt; <a href="/api/pax-runtime-api/drawing.md#gradientstop">GradientStop</a></code></pre>

Constructs a gradient stop at `position`.

##### `with_alpha_factor`
<pre><code class="api-signature language-rust ignore">pub fn with_alpha_factor(&amp;self, factor: f64) -&gt; <a href="/api/pax-runtime-api/drawing.md#gradientstop">GradientStop</a></code></pre>

Returns a copy of this stop with alpha multiplied by `factor`.

---

### `LinearGradient`
Describes a linear gradient fill with a start and end point, and a list of color stops.

Pax templates canonically author each point as `[x, y]`: magic index `0`
is the horizontal coordinate and index `1` is the vertical coordinate.

#### Properties
##### `start`
Type: ([`Size`](/api/pax-runtime-api/layout.md#size), [`Size`](/api/pax-runtime-api/layout.md#size))

Gradient start point in the primitive's local coordinate space.

##### `end`
Type: ([`Size`](/api/pax-runtime-api/layout.md#size), [`Size`](/api/pax-runtime-api/layout.md#size))

Gradient end point in the primitive's local coordinate space.

##### `stops`
Type: `Vec`<[`GradientStop`](/api/pax-runtime-api/drawing.md#gradientstop)>

Ordered color stops along the gradient.

---

### `MaterialParams`
Tunable response parameters for a light-reactive vector material.

#### Properties
##### `ambient`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<`f64`>

Ambient contribution multiplier.

##### `diffuse`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<`f64`>

Diffuse contribution multiplier.

##### `specular`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<`f64`>

Specular contribution multiplier.

##### `roughness`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<`f64`>

Surface roughness in the `[0.0, 1.0]` range.

##### `metallic`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<`f64`>

Metallic response in the `[0.0, 1.0]` range.

##### `emissive`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<[`Color`](/api/pax-runtime-api/color.md#color)>

Additive emissive color.

##### `emissive_intensity`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<`f64`>

Additive emissive intensity.

---

### `RadialGradient`
Describes a radial gradient fill with a start and end point, a radius, and a list of color stops.

Pax templates canonically author each point as `[x, y]`: magic index `0`
is the horizontal coordinate and index `1` is the vertical coordinate.

#### Properties
##### `end`
Type: ([`Size`](/api/pax-runtime-api/layout.md#size), [`Size`](/api/pax-runtime-api/layout.md#size))

Outer radius endpoint in the primitive's local coordinate space.

##### `start`
Type: ([`Size`](/api/pax-runtime-api/layout.md#size), [`Size`](/api/pax-runtime-api/layout.md#size))

Gradient center point in the primitive's local coordinate space.

##### `radius`
Type: `f64`

Radial gradient radius.

##### `stops`
Type: `Vec`<[`GradientStop`](/api/pax-runtime-api/drawing.md#gradientstop)>

Ordered color stops along the gradient.

---

### `SceneAmbientLight`
Resolved ambient light for one logical canvas layer.

#### Properties
##### `color`
Type: [`Color`](/api/pax-runtime-api/color.md#color)

Ambient color.

##### `intensity`
Type: `f64`

Ambient intensity.

---

### `SceneLight`
Resolved light contribution for one logical canvas layer.

#### Properties
##### `shape`
Type: [`LightShape`](/api/pax-runtime-api/drawing.md#lightshape)

Positional or directional light shape.

##### `position`
Type: [`Vector3`](/api/pax-runtime-api/drawing.md#vector3)

Position in logical canvas pixels for point lights.

##### `direction`
Type: [`Vector3`](/api/pax-runtime-api/drawing.md#vector3)

Direction in scene space for directional lights.

##### `color`
Type: [`Color`](/api/pax-runtime-api/color.md#color)

Light color.

##### `intensity`
Type: `f64`

Light intensity.

##### `radius`
Type: `f64`

Point light radius in logical pixels.

##### `enabled`
Type: `bool`

Whether this light contributes.

---

### `SceneLighting`
Resolved lighting state for one logical canvas layer.

#### Properties
##### `active`
Type: `bool`

Whether authored lights or ambient overrides are present.

##### `ambient_is_authored`
Type: `bool`

Whether `ambient` came from an enabled authored `AmbientLight`.

The default ambient is only applied to primitives with at least one
eligible direct light. An authored ambient remains layer-wide.

##### `ambient`
Type: [`SceneAmbientLight`](/api/pax-runtime-api/drawing.md#sceneambientlight)

Singleton ambient contribution.

##### `lights`
Type: `Vec`<[`SceneLight`](/api/pax-runtime-api/drawing.md#scenelight)>

Positional and directional light contributions.

#### Implementations
###### `DEFAULT_AMBIENT_INTENSITY`
Ambient intensity used when point/directional lights exist but no explicit ambient override exists.

###### `MAX_LIGHTS`
Maximum number of simultaneously enabled lights in one target canvas layer.

##### `identity`
<pre><code class="api-signature language-rust ignore">pub fn identity() -&gt; Self</code></pre>

Returns lighting that preserves unlit legacy rendering.

##### `with_default_ambient`
<pre><code class="api-signature language-rust ignore">pub fn with_default_ambient(lights: Vec&lt;<a href="/api/pax-runtime-api/drawing.md#scenelight">SceneLight</a>&gt;) -&gt; Self</code></pre>

Creates active lighting with Pax's default ambient term.

---

### `Stroke`
Describes the outline drawn around vector geometry.

Pax currently renders strokes centered on the underlying path. For open
geometry, `cap` controls how the stroke terminates at the start and end of
the path, while `join` controls how adjacent segments meet.

#### Properties
##### `color`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<[`Color`](/api/pax-runtime-api/color.md#color)>

The stroke color, including alpha.

##### `width`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<[`Size`](/api/pax-runtime-api/layout.md#size)>

The stroke width.

The type is [`Size`] for consistency with the wider property system, but
current vector renderers interpret this value in pixels.

##### `cap`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<[`StrokeCap`](/api/pax-runtime-api/drawing.md#strokecap)>

The cap style used for exposed endpoints on open paths.

##### `join`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<[`StrokeJoin`](/api/pax-runtime-api/drawing.md#strokejoin)>

The join style used where adjacent stroke segments meet.

---

### `Vector3`
A three-dimensional vector in logical scene space.

#### Properties
##### `x`
Type: `f64`

X component.

##### `y`
Type: `f64`

Y component.

##### `z`
Type: `f64`

Z component.

#### Implementations
##### `new`
<pre><code class="api-signature language-rust ignore">pub fn new(x: f64, y: f64, z: f64) -&gt; Self</code></pre>

Constructs a 3D vector.

## Enums
### `Fill`
Describes how to fill vector geometry.

#### Variants
##### `Solid`([`Color`](/api/pax-runtime-api/color.md#color))
A single solid color.

##### `LinearGradient`([`LinearGradient`](/api/pax-runtime-api/drawing.md#lineargradient))
A linear gradient.

##### `RadialGradient`([`RadialGradient`](/api/pax-runtime-api/drawing.md#radialgradient))
A radial gradient.

#### Implementations
##### `coverage_alpha_0_1`
<pre><code class="api-signature language-rust ignore">pub fn coverage_alpha_0_1(&amp;self) -&gt; f64</code></pre>

Estimates the alpha coverage contributed by this fill.

##### `linearGradient`
<pre><code class="api-signature language-rust ignore">pub fn linearGradient(start: (<a href="/api/pax-runtime-api/layout.md#size">Size</a>, <a href="/api/pax-runtime-api/layout.md#size">Size</a>), end: (<a href="/api/pax-runtime-api/layout.md#size">Size</a>, <a href="/api/pax-runtime-api/layout.md#size">Size</a>), stops: Vec&lt;<a href="/api/pax-runtime-api/drawing.md#gradientstop">GradientStop</a>&gt;) -&gt; <a href="/api/pax-runtime-api/drawing.md#fill">Fill</a></code></pre>

Constructs a linear gradient fill.

Pax templates should normally use `@gradient`. When this helper is
needed explicitly, pass `start` and `end` as `[x, y]` lists.

##### `max_alpha_0_1`
<pre><code class="api-signature language-rust ignore">pub fn max_alpha_0_1(&amp;self) -&gt; f64</code></pre>

Returns the maximum alpha used by this fill.

##### `with_alpha_factor`
<pre><code class="api-signature language-rust ignore">pub fn with_alpha_factor(&amp;self, factor: f64) -&gt; <a href="/api/pax-runtime-api/drawing.md#fill">Fill</a></code></pre>

Returns a copy of this fill with alpha multiplied by `factor`.

---

### `LightShape`
Shape of a light contribution in logical scene space.

#### Variants
##### `Point`
A positional light with radius-based attenuation.

##### `Directional`
A light with direction but no position or attenuation.

---

### `Material`
Light-reactive surface response for vector primitives.

This is intentionally named `Material`; `texture` is reserved for future
bitmap-backed texture maps and pattern data.

#### Variants
##### `Lit`([`MaterialParams`](/api/pax-runtime-api/drawing.md#materialparams))
Responds to scene lighting using parameterized material coefficients.

##### `Unlit`
Ignores scene lighting and preserves legacy unlit rendering behavior.

#### Implementations
##### `custom`
<pre><code class="api-signature language-rust ignore">pub fn custom(params: <a href="/api/pax-runtime-api/drawing.md#materialparams">MaterialParams</a>) -&gt; Self</code></pre>

Creates a lit material from explicit coefficients.

##### `emissive`
<pre><code class="api-signature language-rust ignore">pub fn emissive(color: <a href="/api/pax-runtime-api/color.md#color">Color</a>, intensity: f64) -&gt; Self</code></pre>

An emissive material that adds color independent of lights.

##### `glossy`
<pre><code class="api-signature language-rust ignore">pub fn glossy(specular: f64) -&gt; Self</code></pre>

A higher-specular material with lower roughness.

##### `matte`
<pre><code class="api-signature language-rust ignore">pub fn matte() -&gt; Self</code></pre>

A soft, low-specular material suitable as the default lit response.

##### `metallic`
<pre><code class="api-signature language-rust ignore">pub fn metallic(metallic: f64) -&gt; Self</code></pre>

A metallic material response.

##### `unlit`
<pre><code class="api-signature language-rust ignore">pub fn unlit() -&gt; Self</code></pre>

A material that ignores authored lights.

---

### `NavigationTarget`
Describes where to open new windows or tabs when navigating to a URL from a `Link` node.

#### Variants
##### `Current`
Navigate in the current window or tab.

##### `New`
Navigate in a new window or tab.

---

### `PathElement`
Describes a single element of a vector path, such as a line, a point, or curve segment.

#### Variants
##### `Empty`
No-op path element.

##### `Point`([`Size`](/api/pax-runtime-api/layout.md#size), [`Size`](/api/pax-runtime-api/layout.md#size))
Moves the current point to the provided coordinate.

##### `Line`
Draws a straight line to the following `Point`.

##### `Quadratic`([`Size`](/api/pax-runtime-api/layout.md#size), [`Size`](/api/pax-runtime-api/layout.md#size))
Draws a quadratic Bézier segment with one control point, ending at the following `Point`.

##### `Cubic`([`Size`](/api/pax-runtime-api/layout.md#size), [`Size`](/api/pax-runtime-api/layout.md#size), [`Size`](/api/pax-runtime-api/layout.md#size), [`Size`](/api/pax-runtime-api/layout.md#size))
Draws a cubic Bézier segment with two control points, ending at the following `Point`.

##### `Close`
Closes the current contour.

---

### `PathSmoothing`
Controls optional curve smoothing for path geometry before tessellation.

`PathSmoothing` is intended for authored or imported paths whose source data
approximates curves with many short line segments, such as single-stroke SVG
fonts. Existing paths keep their exact geometry by default.

#### Variants
##### `None`
Preserve the authored path exactly.

##### `Light`
Lightly smooth polyline runs while preserving sharp corners.

##### `Strong`
More aggressively smooth polyline runs for pen-like paths.

---

### `StrokeCap`
Controls how an open stroke terminates at the exposed endpoints of a path.

`StrokeCap` affects primitives such as `Line` and open `Path` subpaths.
Closed geometry ignores cap style because it has no exposed endpoints.

#### Variants
##### `Butt`
Ends exactly at the path endpoint without extending past it.

##### `Round`
Adds a semicircular cap whose radius is half the stroke width.

##### `Square`
Adds a square cap that extends half the stroke width past the endpoint.

---

### `StrokeJoin`
Controls how stroke segments are joined at path vertices.

#### Variants
##### `Miter`
Extends outer edges to a point, subject to the renderer's miter limit.

##### `Round`
Rounds the outside of each join.

##### `Bevel`
Cuts joins off with a straight edge.
