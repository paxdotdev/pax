# drawing::rectangle
<!-- summary: API docs for pax-std::drawing::rectangle. -->
<!-- tags: api, pax-std -->

## Structs
### `Rectangle`
A 2D vector rectangle, which covers its bounding box with the specified fill and stroke.

#### Properties
##### `stroke`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<[`Stroke`](/api/pax-runtime-api/drawing.md#stroke)>

Stroke drawn around the rectangle.

##### `fill`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<[`Fill`](/api/pax-runtime-api/drawing.md#fill)>

Fill painted inside the rectangle.

##### `material`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<[`Material`](/api/pax-runtime-api/drawing.md#material)>

Light-reactive surface response.

##### `corner_radii`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<[`RectangleCornerRadii`](/api/pax-std/drawing/rectangle.md#rectanglecornerradii)>

Per-corner radii.

---

### `RectangleCornerRadii`
Corner radii for a rectangle, ordered clockwise from top-left.

#### Properties
##### `top_left`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<[`Numeric`](/api/pax-runtime-api/pax_value/numeric.md#numeric)>

Top-left corner radius.

##### `top_right`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<[`Numeric`](/api/pax-runtime-api/pax_value/numeric.md#numeric)>

Top-right corner radius.

##### `bottom_right`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<[`Numeric`](/api/pax-runtime-api/pax_value/numeric.md#numeric)>

Bottom-right corner radius.

##### `bottom_left`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<[`Numeric`](/api/pax-runtime-api/pax_value/numeric.md#numeric)>

Bottom-left corner radius.

#### Implementations
##### `radii`
<pre><code class="api-signature language-rust ignore">pub fn radii(top_left: <a href="/api/pax-runtime-api/pax_value/numeric.md#numeric">Numeric</a>, top_right: <a href="/api/pax-runtime-api/pax_value/numeric.md#numeric">Numeric</a>, bottom_right: <a href="/api/pax-runtime-api/pax_value/numeric.md#numeric">Numeric</a>, bottom_left: <a href="/api/pax-runtime-api/pax_value/numeric.md#numeric">Numeric</a>) -&gt; Self</code></pre>

Constructs a `RectangleCornerRadii` value from clockwise corner radii.
