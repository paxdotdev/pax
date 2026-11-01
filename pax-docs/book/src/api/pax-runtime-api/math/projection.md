# math::projection
<!-- summary: Homogeneous flat-surface projection shared by rendering, native controls and input. -->
<!-- tags: api, pax-runtime-api -->

Homogeneous flat-surface projection shared by rendering, native controls and input.

## Structs
### `PlanarProjection`
A column-major homography from local x/y to presentation x/y.

Coordinates are divided by the third homogeneous coordinate explicitly.
This describes a flat surface, not its layout or its camera clipping range.

#### Properties
##### `0`
Type: [`f64`; 9]

#### Implementations
##### `as_affine`
<pre><code class="api-signature language-rust ignore">pub fn as_affine(self) -&gt; Option&lt;Affine&gt;</code></pre>

Returns an affine only when this mapping is exactly affine.

##### `homogeneous`
<pre><code class="api-signature language-rust ignore">pub fn homogeneous(self, point: Point) -&gt; [f64; 3]</code></pre>

Multiplies a point without dividing or applying camera clipping.

##### `inverse`
<pre><code class="api-signature language-rust ignore">pub fn inverse(self) -&gt; Option&lt;Self&gt;</code></pre>

Inverts a nonsingular finite plane mapping, preserving homogeneous sign.

##### `matrix3d`
<pre><code class="api-signature language-rust ignore">pub fn matrix3d(self) -&gt; [f64; 16]</code></pre>

Embeds the plane in a CSS/Apple column-major 4x4 matrix.
Depth ordering is supplied separately by the shared compositor.

##### `project`
<pre><code class="api-signature language-rust ignore">pub fn project(self, point: Point) -&gt; Option&lt;Point&gt;</code></pre>

Projects a finite point on the positive-w side of the mapping.

##### `project_path`
<pre><code class="api-signature language-rust ignore">pub fn project_path(self, path: &amp;BezPath, tolerance: f64) -&gt; BezPath</code></pre>

Transforms a path whose vertices lie in the finite positive-w chart.
Curves are flattened using a caller-supplied local-space tolerance.

---

### `ProjectedPlane`
A projected local z=0 surface, including camera depth clipping.

The matrix outputs logical-pixel x/y multiplied by w. In perspective mode,
depth is 0 at the far plane and w at the near plane, so larger depth remains
nearer in Pax's compositor. Orthographic surfaces retain unbounded depth.

#### Implementations
##### `affine`
<pre><code class="api-signature language-rust ignore">pub fn affine(transform: Affine) -&gt; Self</code></pre>

Creates an unbounded-depth affine plane.

##### `camera_clip_rows`
<pre><code class="api-signature language-rust ignore">pub fn camera_clip_rows(self) -&gt; [[f64; 3]; 2]</code></pre>

Camera reverse-depth and w equations, retained across surface rebasing.

##### `clip_polygon`
<pre><code class="api-signature language-rust ignore">pub fn clip_polygon(self, polygon: Vec&lt;Point&gt;, viewport: Option&lt;Rect&gt;) -&gt; Vec&lt;Point&gt;</code></pre>

Clips a local convex polygon in homogeneous coordinates before division.
Viewport clipping bounds exposure even when a surface crosses the eye plane.

##### `clips_depth`
<pre><code class="api-signature language-rust ignore">pub fn clips_depth(self) -&gt; bool</code></pre>

Whether the camera imposes a finite near/far depth interval.

##### `contains_local`
<pre><code class="api-signature language-rust ignore">pub fn contains_local(self, point: Point) -&gt; bool</code></pre>

Tests the camera clip interval before division. Layout bounds are separate.

##### `coverage`
<pre><code class="api-signature language-rust ignore">pub fn coverage(self, bounds: Rect, viewport: Option&lt;Rect&gt;) -&gt; Option&lt;Rect&gt;</code></pre>

Conservative finite coverage after camera and optional viewport clipping.

##### `depth_plane`
<pre><code class="api-signature language-rust ignore">pub fn depth_plane(self) -&gt; Option&lt;<a href="../../../api/pax-runtime-api/rendering.md#depthplane">DepthPlane</a>&gt;</code></pre>

Screen-space depth equation for a noncollapsed projected plane.

##### `footprint`
<pre><code class="api-signature language-rust ignore">pub fn footprint(self, bounds: Rect, viewport: Option&lt;Rect&gt;) -&gt; Vec&lt;Point&gt;</code></pre>

Visible convex footprint in presentation coordinates.

##### `local_point`
<pre><code class="api-signature language-rust ignore">pub fn local_point(self, point: Point) -&gt; Option&lt;Point&gt;</code></pre>

Inverse projection and camera clipping agree, including near-plane crossings.

##### `magnification_bound`
<pre><code class="api-signature language-rust ignore">pub fn magnification_bound(self, bounds: Rect, viewport: Option&lt;Rect&gt;) -&gt; Option&lt;f64&gt;</code></pre>

Conservative upper bound on local magnification over the visible polygon.
The denominator is bounded over the whole clipped region, not sampled at
its center. Hosts still apply their normal backing-density and pixel caps.

##### `mapping`
<pre><code class="api-signature language-rust ignore">pub fn mapping(self) -&gt; <a href="../../../api/pax-runtime-api/math/projection.md#planarprojection">PlanarProjection</a></code></pre>

Plane homography used for drawing and inverse input mapping.

##### `matrix`
<pre><code class="api-signature language-rust ignore">pub fn matrix(self) -&gt; <a href="../../../api/pax-runtime-api/math/spatial.md#transform3">Transform3</a>&lt;<a href="../../../api/pax-runtime-api/math.md#generic">Generic</a>&gt;</code></pre>

Complete homogeneous matrix, retaining near/far clipping coordinates.

##### `new`
<pre><code class="api-signature language-rust ignore">pub fn new&lt;F: <a href="../../../api/pax-runtime-api/math.md#space">Space</a>, T: <a href="../../../api/pax-runtime-api/math.md#space">Space</a>&gt;(matrix: <a href="../../../api/pax-runtime-api/math/spatial.md#transform3">Transform3</a>&lt;F, T&gt;, clip_depth: bool) -&gt; Option&lt;Self&gt;</code></pre>

Retains a finite projection, including collapsed or edge-on planes.

##### `post_affine`
<pre><code class="api-signature language-rust ignore">pub fn post_affine(self, transform: Affine) -&gt; Self</code></pre>

Composes an affine change of destination coordinates, preserving depth and w.

##### `post_mapping`
<pre><code class="api-signature language-rust ignore">pub fn post_mapping(self, mapping: <a href="../../../api/pax-runtime-api/math/projection.md#planarprojection">PlanarProjection</a>) -&gt; Self</code></pre>

Expresses this surface in another projected plane's coordinate chart.
Camera clip equations remain in the original local coordinates.

##### `pre_affine`
<pre><code class="api-signature language-rust ignore">pub fn pre_affine(self, transform: Affine) -&gt; Self</code></pre>

Composes local geometry before projection, including local scroll offsets.

##### `project_path`
<pre><code class="api-signature language-rust ignore">pub fn project_path(self, path: &amp;BezPath, viewport: Option&lt;Rect&gt;) -&gt; BezPath</code></pre>

Projects filled contours after camera/viewport clipping. Curves use a
conservative magnification bound to keep flattening error within a
quarter presentation pixel, including near-plane intersections.
