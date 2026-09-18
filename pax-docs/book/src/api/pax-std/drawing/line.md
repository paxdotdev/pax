# drawing::line
<!-- summary: API docs for pax-std::drawing::line. -->
<!-- tags: api, pax-std -->

## Structs
### `Line`
A 2D vector line segment.

`x1`/`y1` and `x2`/`y2` describe the segment endpoints in the primitive's
local coordinate space. The segment is rendered with `stroke`, whose cap
style controls how the two exposed endpoints terminate.

#### Properties
##### `x1`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<[`Size`](../../../api/pax-runtime-api/layout.md#size)>

The x-coordinate of the start point.

##### `y1`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<[`Size`](../../../api/pax-runtime-api/layout.md#size)>

The y-coordinate of the start point.

##### `x2`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<[`Size`](../../../api/pax-runtime-api/layout.md#size)>

The x-coordinate of the end point.

##### `y2`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<[`Size`](../../../api/pax-runtime-api/layout.md#size)>

The y-coordinate of the end point.

##### `stroke`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<[`Stroke`](../../../api/pax-runtime-api/drawing.md#stroke)>

The stroke used to render the segment.

##### `material`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<[`Material`](../../../api/pax-runtime-api/drawing.md#material)>

Light-reactive surface response.
