# core::frame
<!-- summary: API docs for pax-std::core::frame. -->
<!-- tags: api, pax-std -->

## Structs
### `Frame`
A primitive that gathers children underneath a single render node with a shared base transform,
like `Group`, except `Frame` has the option of clipping rendering outside
of its bounds.

If clipping or the option of clipping is not required,
a `Group` will generally be a more performant and otherwise-equivalent
to `Frame`, since `Frame` creates a clipping mask.

#### Properties
##### `autosize`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<`bool`>

Automatically sizes the frame to its direct content children when possible.

##### `autosize_x`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<`Option`<`bool`>>

Optional override for whether autosize manages the `x` axis.

##### `autosize_y`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<`Option`<`bool`>>

Optional override for whether autosize manages the `y` axis.

##### `corner_radius`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<`f64`>

Corner radius used for the frame clipping mask, in pixels.
