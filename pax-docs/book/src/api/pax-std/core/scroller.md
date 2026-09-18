# core::scroller
<!-- summary: API docs for pax-std::core::scroller. -->
<!-- tags: api, pax-std -->

## Structs
### `Scroller`
A scrolling container, which clips its bounds and offers platform-native scrolling
on either or both of the horizontal and vertical axes.

#### Properties
##### `scroll_pos_x`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<`f64`>

Horizontal scroll offset, in pixels.

##### `scroll_pos_y`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<`f64`>

Vertical scroll offset, in pixels.

##### `scroll_width`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<[`Size`](../../../api/pax-runtime-api/layout.md#size)>

Width of the scrollable content pane.

##### `scroll_height`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<[`Size`](../../../api/pax-runtime-api/layout.md#size)>

Height of the scrollable content pane.

##### `autosize`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<`bool`>

Automatically sizes the scroll pane on its default axes when possible.

For `Scroller`, the default autosized axis is `y`; `x` remains bound to
`scroll_width` unless `autosize_x` explicitly opts in.

##### `autosize_x`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<`Option`<`bool`>>

Optional override for whether autosize manages the `x` axis.

##### `autosize_y`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<`Option`<`bool`>>

Optional override for whether autosize manages the `y` axis.

##### `corner_radius`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<`f64`>

Corner radius for the scroller clipping region, in pixels.

##### `snap_positions_x`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<`Vec`<[`Size`](../../../api/pax-runtime-api/layout.md#size)>>

Scroll snap anchors expressed in px/% along each axis.
Web maps to CSS scroll-snap-type + scroll-snap-align; Apple chassis map these
offsets to native scroll end-points while keeping engine scroll state authoritative.

##### `snap_positions_y`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<`Vec`<[`Size`](../../../api/pax-runtime-api/layout.md#size)>>

Vertical scroll snap anchors expressed in px/%.
