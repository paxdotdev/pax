# drawing::ellipse
<!-- summary: API docs for pax-std::drawing::ellipse. -->
<!-- tags: api, pax-std -->

## Structs
### `Ellipse`
A 2D vector ellipse, which inscribes its bounding box with the specified fill and stroke.

#### Properties
##### `stroke`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<`Vec`<[`Stroke`](../../../api/pax-runtime-api/drawing.md#stroke)>>

Ordered outline layers above the fills, index zero topmost. Empty by default.

##### `fill`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<`Vec`<[`Fill`](../../../api/pax-runtime-api/drawing.md#fill)>>

Paint painted inside the ellipse.
