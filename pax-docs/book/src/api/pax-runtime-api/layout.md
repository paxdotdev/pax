# layout
<!-- summary: Layout units, axes, and common properties shared by renderable nodes. -->
<!-- tags: api, pax-runtime-api -->

Layout units, axes, and common properties shared by renderable nodes.

## Structs
### `CommonProperties`
Properties shared by every renderable Pax node.

Each property here is special-cased by the compiler when parsing element properties,
for example `<SomeElement width={...} />`.

#### Properties
##### `id`
Type: [`Property`](../../api/pax-runtime-api/properties.md#property)<`Option`<`String`>>

Optional stable node identifier.

##### `x`
Type: [`Property`](../../api/pax-runtime-api/properties.md#property)<`Option`<[`Size`](../../api/pax-runtime-api/layout.md#size)>>

Horizontal position.

##### `y`
Type: [`Property`](../../api/pax-runtime-api/properties.md#property)<`Option`<[`Size`](../../api/pax-runtime-api/layout.md#size)>>

Vertical position.

##### `padding_x`
Type: [`Property`](../../api/pax-runtime-api/properties.md#property)<`Option`<[`Size`](../../api/pax-runtime-api/layout.md#size)>>

Symmetric inner spacing applied to this node's child layout area on the x axis.

##### `padding_y`
Type: [`Property`](../../api/pax-runtime-api/properties.md#property)<`Option`<[`Size`](../../api/pax-runtime-api/layout.md#size)>>

Symmetric inner spacing applied to this node's child layout area on the y axis.

##### `width`
Type: [`Property`](../../api/pax-runtime-api/properties.md#property)<`Option`<[`Size`](../../api/pax-runtime-api/layout.md#size)>>

Horizontal extent.

##### `height`
Type: [`Property`](../../api/pax-runtime-api/properties.md#property)<`Option`<[`Size`](../../api/pax-runtime-api/layout.md#size)>>

Vertical extent.

##### `anchor_x`
Type: [`Property`](../../api/pax-runtime-api/properties.md#property)<`Option`<[`Size`](../../api/pax-runtime-api/layout.md#size)>>

Horizontal transform origin, relative to the node's own bounds.

##### `anchor_y`
Type: [`Property`](../../api/pax-runtime-api/properties.md#property)<`Option`<[`Size`](../../api/pax-runtime-api/layout.md#size)>>

Vertical transform origin, relative to the node's own bounds.

##### `scale_x`
Type: [`Property`](../../api/pax-runtime-api/properties.md#property)<`Option`<[`Size`](../../api/pax-runtime-api/layout.md#size)>>

Horizontal scale coefficient.

##### `scale_y`
Type: [`Property`](../../api/pax-runtime-api/properties.md#property)<`Option`<[`Size`](../../api/pax-runtime-api/layout.md#size)>>

Vertical scale coefficient.

##### `skew_x`
Type: [`Property`](../../api/pax-runtime-api/properties.md#property)<`Option`<[`Rotation`](../../api/pax-runtime-api/transform.md#rotation)>>

Horizontal skew.

##### `skew_y`
Type: [`Property`](../../api/pax-runtime-api/properties.md#property)<`Option`<[`Rotation`](../../api/pax-runtime-api/transform.md#rotation)>>

Vertical skew.

##### `rotate`
Type: [`Property`](../../api/pax-runtime-api/properties.md#property)<`Option`<[`Rotation`](../../api/pax-runtime-api/transform.md#rotation)>>

Rotation around the z axis.

##### `transform`
Type: [`Property`](../../api/pax-runtime-api/properties.md#property)<`Option`<[`Transform2D`](../../api/pax-runtime-api/transform.md#transform2d)>>

Full composed transform.

##### `opacity`
Type: [`Property`](../../api/pax-runtime-api/properties.md#property)<`Option`<[`Opacity`](../../api/pax-runtime-api/color.md#opacity)>>

Node opacity, applied to the node and its descendants.

##### `layout_role`
Type: [`Property`](../../api/pax-runtime-api/properties.md#property)<`Option`<[`LayoutRole`](../../api/pax-runtime-api/layout.md#layoutrole)>>

Controls whether this node participates in parent layout measurement and flow.

##### `unclippable`
Type: [`Property`](../../api/pax-runtime-api/properties.md#property)<`Option`<`bool`>>

Allows a node to render outside an ancestor clipping frame.

## Enums
### `Axis`
Model of 2D cartesian axes, used to disambiguate calculations that depend on axis direction (e.g. width vs height)

#### Variants
##### `X`
Horizontal axis.

##### `Y`
Vertical axis.

---

### `LayoutRole`
Controls whether a node participates in parent layout measurement.

#### Variants
##### `Default`
Normal layout behavior. The node contributes to parent hulls, autosize, and flow.

##### `Breakout`
Parent-local positioning that does not contribute to parent hulls, autosize, or flow.

`Breakout` remains in the normal render, hit-test, scroll, and clipping trees; it does
not portal above ancestor frames, masks, or scrollers.

---

### `Size`
A spatial size value that can be either a concrete pixel value like `25px`, a percent of parent bounds like `50%`,
or an additive/subtractive combination of the two like `(100% - 10px)`.

#### Variants
##### `Pixels`([`Numeric`](../../api/pax-runtime-api/pax_value/numeric.md#numeric))
Concrete pixel length, such as `25px`.

##### `Percent`([`Numeric`](../../api/pax-runtime-api/pax_value/numeric.md#numeric))
Percent length relative to the relevant parent bound, such as `50%`.

##### `Combined`([`Numeric`](../../api/pax-runtime-api/pax_value/numeric.md#numeric), [`Numeric`](../../api/pax-runtime-api/pax_value/numeric.md#numeric))
Additive pixel and percent components, such as `100% - 10px`.

#### Implementations
##### `ZERO`
<pre><code class="api-signature language-rust ignore">pub fn ZERO() -&gt; Self</code></pre>

Returns a zero-pixel size.

##### `expect_percent`
<pre><code class="api-signature language-rust ignore">pub fn expect_percent(&amp;self) -&gt; f64</code></pre>

Returns the wrapped percent value normalized as a float, such that 100% => 1.0.
Panics if wrapped type is not a percentage.

##### `expect_pixels`
<pre><code class="api-signature language-rust ignore">pub fn expect_pixels(&amp;self) -&gt; <a href="../../api/pax-runtime-api/pax_value/numeric.md#numeric">Numeric</a></code></pre>

Returns the pixel value
Panics if wrapped type is not pixels.

##### `evaluate`
<pre><code class="api-signature language-rust ignore">pub fn evaluate(&amp;self, bounds: (f64, f64), axis: <a href="../../api/pax-runtime-api/layout.md#axis">Axis</a>) -&gt; f64</code></pre>

Evaluate a Size in the context of `bounds` and a target `axis`.
Returns a `Pixel` value as a simple f64; calculates `Percent` with respect to `bounds` & `axis`

##### `get_pixels`
<pre><code class="api-signature language-rust ignore">pub fn get_pixels(&amp;self, parent: f64) -&gt; f64</code></pre>

Resolves this size against a parent extent in pixels.
