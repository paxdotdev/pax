# drawing::rectangle
<!-- summary: API docs for pax-std::drawing::rectangle. -->
<!-- tags: api, pax-std -->

## Structs
### `CornerRadii`
Corner radii, ordered clockwise from top-left.

Pax templates canonically use a list of one to four values such as
`corner_radius=[12, 8, 4]`. Lists expand using the same clockwise arity rules
as CSS `border-radius`; a uniform radius may elide the brackets as
`corner_radius=12`.

The zero-based positional ("magic index") contract depends on list arity:

- `[all]`
- `[top-left/bottom-right, top-right/bottom-left]`
- `[top-left, top-right/bottom-left, bottom-right]`
- `[top-left, top-right, bottom-right, bottom-left]`

A contextual named object remains available as explicit longhand:
`corner_radius={ top_left: 12 top_right: 8 bottom_right: 4 bottom_left: 2 }`.
The fully type-qualified constructor also remains valid when explicit type
syntax is useful: `corner_radius=CornerRadii { top_left: 12 top_right: 8
bottom_right: 4 bottom_left: 2 }`.
Interpolation treats each radius independently; zero produces an angular corner.

#### Properties
##### `top_left`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<[`Numeric`](../../../api/pax-runtime-api/pax_value/numeric.md#numeric)>

Top-left corner radius.

##### `top_right`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<[`Numeric`](../../../api/pax-runtime-api/pax_value/numeric.md#numeric)>

Top-right corner radius.

##### `bottom_right`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<[`Numeric`](../../../api/pax-runtime-api/pax_value/numeric.md#numeric)>

Bottom-right corner radius.

##### `bottom_left`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<[`Numeric`](../../../api/pax-runtime-api/pax_value/numeric.md#numeric)>

Bottom-left corner radius.

#### Implementations
##### `radii`
<pre><code class="api-signature language-rust ignore">pub fn radii(top_left: <a href="../../../api/pax-runtime-api/pax_value/numeric.md#numeric">Numeric</a>, top_right: <a href="../../../api/pax-runtime-api/pax_value/numeric.md#numeric">Numeric</a>, bottom_right: <a href="../../../api/pax-runtime-api/pax_value/numeric.md#numeric">Numeric</a>, bottom_left: <a href="../../../api/pax-runtime-api/pax_value/numeric.md#numeric">Numeric</a>) -&gt; Self</code></pre>

Constructs a `CornerRadii` value from clockwise corner radii.

---

### `Rectangle`
A 2D vector rectangle, which covers its bounding box with the specified fill and stroke.

#### Properties
##### `stroke`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<[`Stroke`](../../../api/pax-runtime-api/drawing.md#stroke)>

Stroke drawn around the rectangle.

##### `fill`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<[`Fill`](../../../api/pax-runtime-api/drawing.md#fill)>

Fill painted inside the rectangle.

##### `material`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<[`Material`](../../../api/pax-runtime-api/drawing.md#material)>

Light-reactive surface response.

##### `corner_radius`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<[`CornerRadii`](../../../api/pax-std/drawing/rectangle.md#cornerradii)>

Per-corner radii.
