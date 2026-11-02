# rendering
<!-- summary: Rendering backend contracts and helpers for drawing Pax scene content. -->
<!-- tags: api, pax-runtime-api -->

Rendering backend contracts and helpers for drawing Pax scene content.

## Structs
### `AlphaMaskPaint`
One vector paint contributing alpha to a mask, in local path coordinates.
Color channels do not affect coverage; fill alpha and opacity are multiplied.

#### Properties
##### `path`
Type: `BezPath`

##### `transform`
Type: `Affine`

##### `fill`
Type: [`Paint`](../../api/pax-runtime-api/drawing.md#paint)

##### `opacity`
Type: `f64`

##### `paint_bounds`
Type: `Rect`

Complete local geometry bounds before stroke expansion or trimming.

##### `composition`
Type: `Option`<(`u32`, `f32`)>

Primitive boundary: compose its paints before applying source-relative opacity.

---

### `DepthPlane`
Depth of a flat surface as `z = x*a + y*b + c` in canvas-content coordinates.
Positive depth is toward the default orthographic viewer. Coplanar surfaces
retain paint order; a spatial back face reverses its planar layer stack.

#### Properties
##### `presentation_layer`
Type: `u32`

Independent presentation domain. Higher domains paint above lower ones
regardless of geometric depth; 0 is the scene and positive values rank
viewport Overlay contributions and temporary transfer presentations.

##### `coefficients`
Type: [`f64`; 3]

##### `backface`
Type: `bool`

#### Implementations
##### `coincides_over`
<pre><code class="api-signature language-rust ignore">pub fn coincides_over(self, other: Self, vertices: impl IntoIterator) -&gt; bool</code></pre>

Tests coincidence over a convex region's vertices, in shared depth units.
This tolerance is independent of a consumer's pixel-distance split tolerance.

##### `compare_point_depths`
<pre><code class="api-signature language-rust ignore">pub fn compare_point_depths(left: f64, right: f64) -&gt; Ordering</code></pre>

Compares depths for point picking, from farther to nearer.

The existing picking policy rounds to steps of `1e-7` in the view's depth
units, then uses a total order. Callers resolve equal keys with
[`Self::coplanar_order`]. Compare only depths from the same view/metric.
This quantization is distinct from the coverage-plane coincidence test;
an epsilon comparator would not provide a transitive ordering.

##### `coplanar_order`
<pre><code class="api-signature language-rust ignore">pub fn coplanar_order(self, authored_order: i64) -&gt; (bool, i64)</code></pre>

Stable back-to-front key for geometrically coincident paint. Larger authored
orders paint later on front faces and earlier on back faces. Opposing faces
at exactly the same depth put front-facing paint last, independently of the
authored index range. Renderers, occlusion masks, and picking share this tie.

##### `depth_at`
<pre><code class="api-signature language-rust ignore">pub fn depth_at(self, x: f64, y: f64) -&gt; f64</code></pre>

Evaluates camera-relative depth at a projected coordinate.

##### `equality_line`
<pre><code class="api-signature language-rust ignore">pub fn equality_line(self, other: Self) -&gt; [f64; 3]</code></pre>

Equality line in a common projected chart, before any pixel-distance
normalization. Its positive side is where `self` is nearer than `other`.

##### `front_region`
<pre><code class="api-signature language-rust ignore">pub fn front_region(front: Self, back: Self, front_order: i32, back_order: i32, bounds: Rect) -&gt; Option&lt;BezPath&gt;</code></pre>

The part of `bounds` where `front` precedes `back` in a common projected coordinate chart.
Coplanar paint uses the same facing/order key as the renderer and pointer tests.
The corner depths use a `1e-7` coincidence tolerance in their shared view's
depth units; crossing planes are split at their exact equality line.

---

### `LightingSurface`
Unprojected lighting frame of a planar draw, in logical world pixels.
Geometry stays local: `origin + x * tangent_x + y * tangent_y` supplies its
shading position independently of canvas tiles, captures, or projection.
The oriented normal is unit length and perpendicular to both tangents.

#### Properties
##### `origin`
Type: [`f32`; 3]

##### `tangent_x`
Type: [`f32`; 3]

##### `tangent_y`
Type: [`f32`; 3]

##### `normal`
Type: [`f32`; 3]

---

### `OpacityScope`
One authored opacity boundary, ordered from the outermost ancestor to the painted node.
Native surfaces can present independently while a canvas backend composes each scope.

#### Properties
##### `node_id`
Type: `u32`

##### `opacity`
Type: `f32`

---

### `ReplayCanvasLayerUpdate`
Replay invalidation for one logical canvas layer.

#### Properties
##### `layer`
Type: `usize`

##### `regions`
Type: `Option`<`Vec`<`Rect`>>

Canvas-content regions requiring replay. The runtime selects nodes from its
shared scene geometry; `None` requests full-layer dirtification.

## Enums
### `Layer`
Render layer selected for a primitive or native surface.

#### Variants
##### `Native`
Platform-native element layer.

##### `NativeNonOccluding`
Platform-native layer that does not participate in occlusion.

##### `Canvas`
GPU/canvas-rendered layer.

##### `DontCare`
Runtime can choose the appropriate layer.

## Traits
### `RenderContext`
The drawing contract shared by Pax runtime backends.

## Functions
### `bez_path_to_svg_path_data`
<pre><code class="api-signature language-rust ignore">pub fn bez_path_to_svg_path_data(path: &amp;BezPath) -&gt; String</code></pre>

Convert a kurbo BezPath to a SVG-friendly drawing string
