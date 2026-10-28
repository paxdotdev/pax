# camera
<!-- summary: Camera math shared by spatial presentation and picking. -->
<!-- tags: api, pax-runtime-api -->

Camera math shared by spatial presentation and picking.

These are Rust geometry contracts, not template camera elements or an active
renderer camera. The current renderer and native-control protocol remain 2D.
Orthographic projection preserves window x/y in logical pixels. Perspective
views look along -z, with x right and y down; clip depth is 0 at the near/front
plane and 1 at the far/back plane. View matrices move the world into camera
space without modifying local layout or scene identity.

```
use pax_runtime_api::{
    camera::{Camera, CameraProjection},
    math::{Point3, Transform3},
};

let projection = CameraProjection::orthographic((800., 600.), 1000., -1000.).unwrap();
let camera: Camera = Camera::new(Transform3::identity(), projection).unwrap();
let screen = camera.project(Point3::new(120., 80., 0.)).unwrap();
assert!((screen.position.x - 120.).abs() < 1e-9);
assert!((screen.position.y - 80.).abs() < 1e-9);
assert_eq!(screen.depth, 0.5);
```

## Structs
### `Camera`
An immutable view/projection pair with a cached inverse for picking.

Changing cameras constructs a new pair; it does not change the scene or native
ownership. This type does not select the runtime's active camera.

#### Implementations
##### `new`
<pre><code class="api-signature language-rust ignore">pub fn new(world_to_view: <a href="../../api/pax-runtime-api/math/spatial.md#transform3">Transform3</a>&lt;W, <a href="../../api/pax-runtime-api/camera.md#cameraspace">CameraSpace</a>&gt;, projection: <a href="../../api/pax-runtime-api/camera.md#cameraprojection">CameraProjection</a>) -&gt; Option&lt;Self&gt;</code></pre>

Combines an affine world-to-camera view with a projection.
Rejects singular, non-finite, or projective view transforms.

##### `pick_plane`
<pre><code class="api-signature language-rust ignore">pub fn pick_plane&lt;L: <a href="../../api/pax-runtime-api/math.md#space">Space</a>&gt;(&amp;self, position: Point2&lt;<a href="../../api/pax-runtime-api/platform.md#window">Window</a>&gt;, local_to_world: <a href="../../api/pax-runtime-api/math/spatial.md#transform3">Transform3</a>&lt;L, W&gt;) -&gt; Option&lt;<a href="../../api/pax-runtime-api/camera.md#planehit">PlaneHit</a>&lt;L, W&gt;&gt;</code></pre>

Intersects the visible camera ray with a surface's local z=0 plane.

Returns `None` for collapsed planes/non-affine transforms, numerically parallel
rays, or intersections outside the camera's depth range. Both faces are
pickable here; the caller owns back-face visibility and logical bounds.

##### `project`
<pre><code class="api-signature language-rust ignore">pub fn project(&amp;self, point: <a href="../../api/pax-runtime-api/math/spatial.md#point3">Point3</a>&lt;W&gt;) -&gt; Option&lt;<a href="../../api/pax-runtime-api/camera.md#projectedpoint">ProjectedPoint</a>&gt;</code></pre>

Projects a world point to logical window pixels and normalized depth.
Rejects points behind the eye or outside the front/back clipping planes.
Window x/y are not clipped, so callers can compute conservative coverage.

##### `unproject`
<pre><code class="api-signature language-rust ignore">pub fn unproject(&amp;self, position: Point2&lt;<a href="../../api/pax-runtime-api/platform.md#window">Window</a>&gt;, depth: f64) -&gt; Option&lt;<a href="../../api/pax-runtime-api/math/spatial.md#point3">Point3</a>&lt;W&gt;&gt;</code></pre>

Maps a window point at normalized depth 0..=1 back into world space.
Depths 0 and 1 give the endpoints of the visible picking ray segment.

##### `world_to_clip`
<pre><code class="api-signature language-rust ignore">pub fn world_to_clip(&amp;self) -&gt; <a href="../../api/pax-runtime-api/math/spatial.md#transform3">Transform3</a>&lt;W, <a href="../../api/pax-runtime-api/camera.md#clipspace">ClipSpace</a>&gt;</code></pre>

Returns the exact matrix used by projection and picking.

---

### `CameraProjection`
A validated projection and its logical-pixel viewport.

#### Implementations
##### `matrix`
<pre><code class="api-signature language-rust ignore">pub fn matrix(self) -&gt; <a href="../../api/pax-runtime-api/math/spatial.md#transform3">Transform3</a>&lt;<a href="../../api/pax-runtime-api/camera.md#cameraspace">CameraSpace</a>, <a href="../../api/pax-runtime-api/camera.md#clipspace">ClipSpace</a>&gt;</code></pre>

Returns the camera-to-clip matrix in the shared spatial representation.

##### `orthographic`
<pre><code class="api-signature language-rust ignore">pub fn orthographic(viewport: (f64, f64), front_z: f64, back_z: f64) -&gt; Option&lt;Self&gt;</code></pre>

Creates the compatibility orthographic projection.

Camera-space (0,0) maps to the window's top-left; width/height map to its
bottom-right. `front_z > back_z` are finite camera-space z coordinates,
not positive distances. Choose a range containing zero for existing 2D
content. This utility does not impose a renderer's default depth range.

##### `perspective`
<pre><code class="api-signature language-rust ignore">pub fn perspective(viewport: (f64, f64), vertical_fov: f64, near: f64, far: f64) -&gt; Option&lt;Self&gt;</code></pre>

Creates a perspective projection looking along camera-space -z.

`vertical_fov` is in radians and must be between 0 and pi; near/far are
finite positive distances with far > near. The optical axis projects to
the center of the viewport. No automatic camera pose or layout is added.

##### `viewport`
<pre><code class="api-signature language-rust ignore">pub fn viewport(self) -&gt; (f64, f64)</code></pre>

Returns the viewport width and height in logical pixels.

---

### `CameraSpace`
Camera coordinates: x right, y down, looking toward -z.

---

### `ClipSpace`
Homogeneous GPU clip coordinates, with visible depth in 0..=w.

---

### `PlaneHit`
An intersection with the infinite local z=0 plane.

The caller still applies authored bounds, apertures, and paint/depth ordering.

#### Properties
##### `local`
Type: `Point2`<`L`>

Intersection in the surface's unscaled local x/y coordinates.

##### `world`
Type: [`Point3`](../../api/pax-runtime-api/math/spatial.md#point3)<`W`>

Intersection in world coordinates.

##### `depth`
Type: `f64`

Normalized depth, comparable with other hits through this camera.

##### `front_facing`
Type: `bool`

Whether the transformed x/y basis faces the camera ray.

---

### `ProjectedPoint`
Window position and normalized GPU depth of a projected point.

#### Properties
##### `position`
Type: `Point2`<[`Window`](../../api/pax-runtime-api/platform.md#window)>

Window coordinates in logical pixels; may lie outside the viewport.

##### `depth`
Type: `f64`

Near/front = 0, far/back = 1. Smaller values are nearer the camera.
