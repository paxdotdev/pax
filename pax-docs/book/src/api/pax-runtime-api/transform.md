# transform
<!-- summary: Rotation and 2D transform types used by layout and rendering. -->
<!-- tags: api, pax-runtime-api -->

Rotation and 2D transform types used by layout and rendering.

## Structs
### `Transform2D`
A sugared representation of an Affine transform combined with an `anchor` layout property.

#### Properties
##### `previous`
Type: `Option`<`Box`<[`Transform2D`](../../api/pax-runtime-api/transform.md#transform2d)>>

Linked list of ancestral Transform2Ds

##### `rotate`
Type: `Option`<[`Rotation`](../../api/pax-runtime-api/transform.md#rotation)>

Represents affine rotation over z axis (single-dimensional for 2D rendering)

##### `translate`
Type: `Option`<[[`Size`](../../api/pax-runtime-api/layout.md#size); 2]>

Represents affine translation across the x-y plane

##### `anchor`
Type: `Option`<[[`Size`](../../api/pax-runtime-api/layout.md#size); 2]>

Represents the alignment of the (0,0) position of this element as it relates to its own bounding box. (origin offset)

##### `scale`
Type: `Option`<[[`Size`](../../api/pax-runtime-api/layout.md#size); 2]>

Represents affine scale coefficients across the x-y plane

##### `skew`
Type: `Option`<[[`Rotation`](../../api/pax-runtime-api/transform.md#rotation); 2]>

Represents affine skew over x and y axes

#### Implementations
##### `anchor`
<pre><code class="api-signature language-rust ignore">pub fn anchor(x: <a href="../../api/pax-runtime-api/layout.md#size">Size</a>, y: <a href="../../api/pax-runtime-api/layout.md#size">Size</a>) -&gt; Self</code></pre>

Transform origin point for this element, relative to its own bounding box.

##### `rotate`
<pre><code class="api-signature language-rust ignore">pub fn rotate(z: <a href="../../api/pax-runtime-api/transform.md#rotation">Rotation</a>) -&gt; Self</code></pre>

Rotation over the z axis.

##### `scale`
<pre><code class="api-signature language-rust ignore">pub fn scale(x: <a href="../../api/pax-runtime-api/layout.md#size">Size</a>, y: <a href="../../api/pax-runtime-api/layout.md#size">Size</a>) -&gt; Self</code></pre>

Scale coefficients over the x-y plane.

##### `translate`
<pre><code class="api-signature language-rust ignore">pub fn translate(x: <a href="../../api/pax-runtime-api/layout.md#size">Size</a>, y: <a href="../../api/pax-runtime-api/layout.md#size">Size</a>) -&gt; Self</code></pre>

Translation over the x-y plane.

## Enums
### `Rotation`
Encodes a rotation in various units

#### Variants
##### `Radians`([`Numeric`](../../api/pax-runtime-api/pax_value/numeric.md#numeric))
Radian units (2π rad for one full rotation)

##### `Degrees`([`Numeric`](../../api/pax-runtime-api/pax_value/numeric.md#numeric))
Degree units (360 deg for one full rotation)

##### `Percent`([`Numeric`](../../api/pax-runtime-api/pax_value/numeric.md#numeric))
Percentage units (100% for one full rotation)

#### Implementations
##### `ZERO`
<pre><code class="api-signature language-rust ignore">pub fn ZERO() -&gt; Self</code></pre>

Returns zero degrees.

##### `get_as_degrees`
<pre><code class="api-signature language-rust ignore">pub fn get_as_degrees(&amp;self) -&gt; f64</code></pre>

Returns the rotation as degrees, regardless of the original unit

##### `get_as_radians`
<pre><code class="api-signature language-rust ignore">pub fn get_as_radians(&amp;self) -&gt; f64</code></pre>

Returns the rotation as radians, regardless of the original unit

##### `to_float_0_1`
<pre><code class="api-signature language-rust ignore">pub fn to_float_0_1(&amp;self) -&gt; f64</code></pre>

Returns a normalized float proportional to `0deg : 0.0 :: 360deg : 1.0`.

For example, `0rad` maps to `0.0`, `100%` maps to `1.0`, and `720deg`
maps to `2.0`.
