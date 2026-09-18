# drawing::path
<!-- summary: API docs for pax-std::drawing::path. -->
<!-- tags: api, pax-std -->

## Structs
### `Path`
A 2D vector path for arbitrary Bézier and line-segment chains.

`elements` describes the path in local coordinates. `fill` paints the
interior of closed contours, while `stroke` paints the path itself; for
open subpaths, the stroke cap controls the exposed endpoints. Path geometry
may draw outside the element's layout bounds; use a `Frame` or `Mask` when
that overflow should be clipped.

#### Properties
##### `elements`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<`Vec`<[`PathElement`](../../../api/pax-runtime-api/drawing.md#pathelement)>>

The path commands and control points, expressed in local coordinates.

##### `stroke`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<[`Stroke`](../../../api/pax-runtime-api/drawing.md#stroke)>

The stroke applied along the path centerline.

##### `fill`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<[`Fill`](../../../api/pax-runtime-api/drawing.md#fill)>

The fill applied to the interior of closed contours.

##### `material`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<[`Material`](../../../api/pax-runtime-api/drawing.md#material)>

Light-reactive surface response.

##### `smoothing`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<[`PathSmoothing`](../../../api/pax-runtime-api/drawing.md#pathsmoothing)>

Optional curve smoothing applied before rendering path geometry.

##### `draw_start`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<[`UnitValue`](../../../api/pax-runtime-api/unit_value.md#unitvalue)>

Start position of the visible stroke range over the path's total length.

##### `draw_end`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<[`UnitValue`](../../../api/pax-runtime-api/unit_value.md#unitvalue)>

End position of the visible stroke range over the path's total length.

#### Implementations
##### `curve_to`
<pre><code class="api-signature language-rust ignore">pub fn curve_to(path: Vec&lt;<a href="../../../api/pax-runtime-api/drawing.md#pathelement">PathElement</a>&gt;, h_x: <a href="../../../api/pax-runtime-api/layout.md#size">Size</a>, h_y: <a href="../../../api/pax-runtime-api/layout.md#size">Size</a>, x: <a href="../../../api/pax-runtime-api/layout.md#size">Size</a>, y: <a href="../../../api/pax-runtime-api/layout.md#size">Size</a>) -&gt; Vec&lt;<a href="../../../api/pax-runtime-api/drawing.md#pathelement">PathElement</a>&gt;</code></pre>

Appends a quadratic Bézier curve with one control point.

##### `line_to`
<pre><code class="api-signature language-rust ignore">pub fn line_to(path: Vec&lt;<a href="../../../api/pax-runtime-api/drawing.md#pathelement">PathElement</a>&gt;, x: <a href="../../../api/pax-runtime-api/layout.md#size">Size</a>, y: <a href="../../../api/pax-runtime-api/layout.md#size">Size</a>) -&gt; Vec&lt;<a href="../../../api/pax-runtime-api/drawing.md#pathelement">PathElement</a>&gt;</code></pre>

Appends a straight line segment to the provided point.

##### `start`
<pre><code class="api-signature language-rust ignore">pub fn start(x: <a href="../../../api/pax-runtime-api/layout.md#size">Size</a>, y: <a href="../../../api/pax-runtime-api/layout.md#size">Size</a>) -&gt; Vec&lt;<a href="../../../api/pax-runtime-api/drawing.md#pathelement">PathElement</a>&gt;</code></pre>

Starts a new path at the provided point.

---

### `PathClose`
Path child component that closes the current contour.

---

### `PathCurve`
Path child component that inserts a quadratic curve control point.

#### Properties
##### `x`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<[`Size`](../../../api/pax-runtime-api/layout.md#size)>

Control point x-coordinate.

##### `y`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<[`Size`](../../../api/pax-runtime-api/layout.md#size)>

Control point y-coordinate.

---

### `PathLine`
Path child component that inserts a straight line segment.

---

### `PathPoint`
Path child component that inserts a `PathElement::Point`.

#### Properties
##### `x`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<[`Size`](../../../api/pax-runtime-api/layout.md#size)>

Point x-coordinate.

##### `y`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<[`Size`](../../../api/pax-runtime-api/layout.md#size)>

Point y-coordinate.
