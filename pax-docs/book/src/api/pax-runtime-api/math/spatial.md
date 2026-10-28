# math::spatial
<!-- summary: Spatial coordinates, homogeneous transforms, and camera-independent planes. -->
<!-- tags: api, pax-runtime-api -->

Spatial coordinates, homogeneous transforms, and camera-independent planes.
Types are also reexported by [`super`] alongside the existing 2D math.

## Structs
### `Plane3`
An oriented geometric plane, independent of camera and compositing scope.

#### Implementations
##### `from_transform`
<pre><code class="api-signature language-rust ignore">pub fn from_transform&lt;L: <a href="../../../api/pax-runtime-api/math.md#space">Space</a>&gt;(transform: <a href="../../../api/pax-runtime-api/math/spatial.md#transform3">Transform3</a>&lt;L, W&gt;) -&gt; Option&lt;Self&gt;</code></pre>

Derives the plane of local z=0 from an affine transform's x/y basis.
Returns `None` for a collapsed plane or non-affine transform.
Collapsing only the z axis does not collapse a zero-thickness surface.

##### `is_coplanar`
<pre><code class="api-signature language-rust ignore">pub fn is_coplanar(&amp;self, other: &amp;Self, distance_tolerance: f64, angular_tolerance: f64) -&gt; bool</code></pre>

Compares geometric planes, accepting opposite faces.

Distance tolerance is in logical pixels; angular tolerance is the sine
of the allowed angle between normals (0..=1). Both must be finite and
nonnegative. This pairwise numerical predicate is not a surface cache key
or permission to flatten content across opacity/clipping boundaries.

##### `normal`
<pre><code class="api-signature language-rust ignore">pub fn normal(&amp;self) -&gt; Vector3&lt;W&gt;</code></pre>

Returns its unit normal; reversing this changes face, not coplanarity.

##### `origin`
<pre><code class="api-signature language-rust ignore">pub fn origin(&amp;self) -&gt; <a href="../../../api/pax-runtime-api/math/spatial.md#point3">Point3</a>&lt;W&gt;</code></pre>

Returns a point on the plane, in logical pixels.

---

### `Point3`
A point in a phantom 3D coordinate space.
Local/world positions use logical pixels; projected clip coordinates are unitless.

#### Properties
##### `x`
Type: `f64`

Horizontal coordinate.

##### `y`
Type: `f64`

Vertical coordinate (positive down in the default view).

##### `z`
Type: `f64`

Depth coordinate (positive toward the default viewer).

#### Implementations
##### `is_finite`
<pre><code class="api-signature language-rust ignore">pub fn is_finite(self) -&gt; bool</code></pre>

Returns whether every coordinate is finite.

##### `new`
<pre><code class="api-signature language-rust ignore">pub fn new(x: f64, y: f64, z: f64) -&gt; Self</code></pre>

Constructs coordinates in the indicated space.

##### `xy`
<pre><code class="api-signature language-rust ignore">pub fn xy(self) -&gt; Point2&lt;W&gt;</code></pre>

Drops z after a caller has chosen the plane or projection.

---

### `Transform3`
A column-major homogeneous 4×4 transform between phantom coordinate spaces.

`parent * child` applies the child first, as with [`Transform2`]. Defaults
are identity. [`Self::from`] embeds an affine 2D transform without changing
x/y or flattening z. Raw coefficients use CSS `matrix3d`/WGSL column order.
Perspective division is explicit and fallible through [`Self::project_point`].
No coefficient interpolation is provided: interpolate authored pose values.

#### Implementations
##### `coeffs`
<pre><code class="api-signature language-rust ignore">pub fn coeffs(self) -&gt; [f64; 16]</code></pre>

Returns column-major coefficients for GPU/native presentation.

##### `identity`
<pre><code class="api-signature language-rust ignore">pub fn identity() -&gt; Self</code></pre>

Returns the identity matrix.

##### `is_affine`
<pre><code class="api-signature language-rust ignore">pub fn is_affine(self) -&gt; bool</code></pre>

Returns whether the matrix is finite and has an affine last row.

##### `new`
<pre><code class="api-signature language-rust ignore">pub fn new(m: [f64; 16]) -&gt; Self</code></pre>

Constructs a matrix in column-major order; validation occurs at use sites.

##### `project_point`
<pre><code class="api-signature language-rust ignore">pub fn project_point(self, point: <a href="../../../api/pax-runtime-api/math/spatial.md#point3">Point3</a>&lt;F&gt;) -&gt; Option&lt;<a href="../../../api/pax-runtime-api/math/spatial.md#point3">Point3</a>&lt;T&gt;&gt;</code></pre>

Transforms and divides by w, rejecting zero w and non-finite results.
This does not clip; camera visibility also requires positive w and depth.

##### `rotate_x`
<pre><code class="api-signature language-rust ignore">pub fn rotate_x(radians: f64) -&gt; Self</code></pre>

Rotates around x in radians; positive angles turn +y toward +z.

##### `rotate_y`
<pre><code class="api-signature language-rust ignore">pub fn rotate_y(radians: f64) -&gt; Self</code></pre>

Rotates around y in radians; positive angles turn +z toward +x.

##### `rotate_z`
<pre><code class="api-signature language-rust ignore">pub fn rotate_z(radians: f64) -&gt; Self</code></pre>

Rotates around z in radians, matching [`Transform2::rotate`].

##### `scale`
<pre><code class="api-signature language-rust ignore">pub fn scale(v: Vector3&lt;T&gt;) -&gt; Self</code></pre>

Constructs independent x/y/z scale factors.

##### `transform_homogeneous`
<pre><code class="api-signature language-rust ignore">pub fn transform_homogeneous(self, point: [f64; 4]) -&gt; [f64; 4]</code></pre>

Transforms a homogeneous coordinate without dividing or clipping.

##### `translate`
<pre><code class="api-signature language-rust ignore">pub fn translate(v: Vector3&lt;T&gt;) -&gt; Self</code></pre>

Constructs a translation, in logical pixels.

##### `try_as_2d`
<pre><code class="api-signature language-rust ignore">pub fn try_as_2d(self) -&gt; Option&lt;Transform2&lt;F, T&gt;&gt;</code></pre>

Recovers the exact 2D fast path only if this matrix leaves z unchanged.
No epsilon is used: small, authored spatial changes must not disappear.

##### `try_inverse`
<pre><code class="api-signature language-rust ignore">pub fn try_inverse(self) -&gt; Option&lt;<a href="../../../api/pax-runtime-api/math/spatial.md#transform3">Transform3</a>&lt;T, F&gt;&gt;</code></pre>

Inverts a finite matrix, rejecting singular or unrepresentable results.

---

### `Vector3`
A geometric vector in a phantom 3D coordinate space.

Unlike the authoring [`crate::drawing::Vector3`] lighting value, this type
carries a coordinate-space marker and does not participate in Pax values.

#### Properties
##### `x`
Type: `f64`

Horizontal component.

##### `y`
Type: `f64`

Vertical component.

##### `z`
Type: `f64`

Depth component.

#### Implementations
##### `is_finite`
<pre><code class="api-signature language-rust ignore">pub fn is_finite(self) -&gt; bool</code></pre>

Returns whether every coordinate is finite.

##### `new`
<pre><code class="api-signature language-rust ignore">pub fn new(x: f64, y: f64, z: f64) -&gt; Self</code></pre>

Constructs coordinates in the indicated space.

##### `cross`
<pre><code class="api-signature language-rust ignore">pub fn cross(self, other: Self) -&gt; Self</code></pre>

Component cross product; the local x/y basis produces the +z normal.

##### `dot`
<pre><code class="api-signature language-rust ignore">pub fn dot(self, other: Self) -&gt; f64</code></pre>

Scalar product in the shared coordinate space.

##### `try_normalize`
<pre><code class="api-signature language-rust ignore">pub fn try_normalize(self) -&gt; Option&lt;Self&gt;</code></pre>

Returns a unit vector, or `None` for a zero or non-finite vector.
