# drawing
<!-- summary: Vector drawing primitives: paths, fills, strokes, caps, and gradients. -->
<!-- tags: api, pax-runtime-api -->

Vector drawing primitives: paths, fills, strokes, caps, and gradients.

## Submodules
- [drawing::stroke_utils](drawing/stroke_utils.md)

## Structs
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

### `RadialGradient`
Describes a radial gradient fill with a start and end point, a radius, and a list of color stops.

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

### `Stroke`
Describes the outline drawn around vector geometry.

Pax currently renders strokes centered on the underlying path. For open
geometry, `cap` controls how the stroke terminates at the start and end of
the path.

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

##### `max_alpha_0_1`
<pre><code class="api-signature language-rust ignore">pub fn max_alpha_0_1(&amp;self) -&gt; f64</code></pre>

Returns the maximum alpha used by this fill.

##### `with_alpha_factor`
<pre><code class="api-signature language-rust ignore">pub fn with_alpha_factor(&amp;self, factor: f64) -&gt; <a href="/api/pax-runtime-api/drawing.md#fill">Fill</a></code></pre>

Returns a copy of this fill with alpha multiplied by `factor`.

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
