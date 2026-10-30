# spatial
<!-- summary: Retained scene geometry, before camera projection or native scrolling. -->
<!-- tags: api, pax-runtime -->

Retained scene geometry, before camera projection or native scrolling.

## Structs
### `SpatialGeometry`
A node's unprojected world transform and local layout dimensions.

World coordinates currently align with the unscrolled window. Composition
retains all three axes; projection is a separate operation. Existing 2D
trees retain the affine multiplication fast path. Bounds remain local and
do not change when a surface tilts or a camera moves.

Common spatial properties populate this retained geometry. The default
orthographic projection supplies exact affine footprints to native controls;
the GPU compositor separately retains plane depth and facing.

#### Properties
##### `bounds`
Type: (`f64`, `f64`)

Untransformed width and height in logical pixels.

#### Implementations
##### `append_local`
<pre><code class="api-signature language-rust ignore">pub fn append_local(self, transform: <a href="../../../api/pax-runtime-api/math/spatial.md#transform3">Transform3</a>&lt;<a href="../../../api/internal/pax-runtime/engine/node_interface.md#nodelocal">NodeLocal</a>, <a href="../../../api/internal/pax-runtime/engine/node_interface.md#nodelocal">NodeLocal</a>&gt;, bounds: (f64, f64)) -&gt; Self</code></pre>

Appends child-local geometry without projecting the parent first.

##### `depth_plane`
<pre><code class="api-signature language-rust ignore">pub fn depth_plane(self, surface: Transform2&lt;<a href="../../../api/internal/pax-runtime/engine/node_interface.md#nodelocal">NodeLocal</a>, <a href="../../../api/pax-runtime-api/platform.md#window">Window</a>&gt;) -&gt; Option&lt;<a href="../../../api/pax-runtime-api/rendering.md#depthplane">DepthPlane</a>&gt;</code></pre>

Retains the depth equation while expressing x/y in a presentation surface.
Returns `None` for the ordinary 2D fast path or an edge-on/invalid surface.

##### `is_coplanar_with`
<pre><code class="api-signature language-rust ignore">pub fn is_coplanar_with(self, other: Self, distance_tolerance: f64, angular_sine_tolerance: f64) -&gt; Option&lt;bool&gt;</code></pre>

Tests geometric coplanarity, independently of front/back orientation.

Tolerances are explicit: distance is in world pixels, angle is the sine
between normals. `None` means at least one plane is undefined. This is
only a geometric test; opacity, clipping, paint order and other isolation
boundaries still govern whether two surfaces may be composited together.

##### `is_spatial`
<pre><code class="api-signature language-rust ignore">pub fn is_spatial(self) -&gt; bool</code></pre>

Whether this value requires the spatial path rather than exact 2D affine arithmetic.

##### `lighting_surface`
<pre><code class="api-signature language-rust ignore">pub fn lighting_surface(self) -&gt; Option&lt;<a href="../../../api/pax-runtime-api/rendering.md#lightingsurface">LightingSurface</a>&gt;</code></pre>

World shading coordinates, independent of orthographic projection and tiles.
Crossed tangents retain the surface's facing under shear and reflection;
ordinary 2D reflections preserve the legacy +Z lighting normal.

##### `new`
<pre><code class="api-signature language-rust ignore">pub fn new(transform: <a href="../../../api/pax-runtime-api/math/spatial.md#transform3">Transform3</a>&lt;<a href="../../../api/internal/pax-runtime/engine/node_interface.md#nodelocal">NodeLocal</a>, <a href="../../../api/pax-runtime-api/platform.md#window">Window</a>&gt;, bounds: (f64, f64)) -&gt; Self</code></pre>

Retains a spatial transform, recovering the 2D fast path only when exact.
Invalid or projective matrices can be retained for inspection, but cannot
produce an affine orthographic presentation or a world plane.

##### `orthographic_projection`
<pre><code class="api-signature language-rust ignore">pub fn orthographic_projection(self) -&gt; Option&lt;<a href="../../../api/internal/pax-runtime/layout.md#transformandbounds">TransformAndBounds</a>&lt;<a href="../../../api/internal/pax-runtime/engine/node_interface.md#nodelocal">NodeLocal</a>, <a href="../../../api/pax-runtime-api/platform.md#window">Window</a>&gt;&gt;</code></pre>

Projects the local z=0 plane onto the default orthographic x/y view.

This preserves the footprint of any finite affine spatial transform,
including an edge-on plane. It deliberately discards depth and must not
be used to compose descendants or determine occlusion. Projective and
non-finite spatial matrices return `None`. Existing planar matrices pass
through unchanged, including the 2D pipeline's handling of invalid values.

##### `plane`
<pre><code class="api-signature language-rust ignore">pub fn plane(self) -&gt; Option&lt;<a href="../../../api/pax-runtime-api/math/spatial.md#plane3">Plane3</a>&lt;<a href="../../../api/pax-runtime-api/platform.md#window">Window</a>&gt;&gt;</code></pre>

Returns the oriented world plane, or `None` for invalid/collapsed x/y bases.
A collapsed z basis does not collapse a flat surface.

##### `world_transform`
<pre><code class="api-signature language-rust ignore">pub fn world_transform(self) -&gt; <a href="../../../api/pax-runtime-api/math/spatial.md#transform3">Transform3</a>&lt;<a href="../../../api/internal/pax-runtime/engine/node_interface.md#nodelocal">NodeLocal</a>, <a href="../../../api/pax-runtime-api/platform.md#window">Window</a>&gt;</code></pre>

Returns the complete transform, including depth and plane orientation.
