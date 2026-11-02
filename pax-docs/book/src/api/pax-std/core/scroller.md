# core::scroller
<!-- summary: API docs for pax-std::core::scroller. -->
<!-- tags: api, pax-std -->

## Structs
### `Scroller`
A scrolling container, which clips its bounds and offers platform-native scrolling
on either or both of the horizontal and vertical axes.

#### Properties
##### `scroll_pos_x`
Type: `Property`<`f64`>

Horizontal scroll offset, in pixels.

##### `scroll_pos_y`
Type: `Property`<`f64`>

Vertical scroll offset, in pixels.

##### `scroll_width`
Type: `Property`<[`Size`](../../../api/pax-runtime-api/layout.md#size)>

Width of the scrollable content pane.

##### `scroll_height`
Type: `Property`<[`Size`](../../../api/pax-runtime-api/layout.md#size)>

Height of the scrollable content pane.

##### `autosize`
Type: `Property`<`bool`>

Automatically sizes the scroll pane on its default axes when possible.

For `Scroller`, the default autosized axis is `y`; `x` remains bound to
`scroll_width` unless `autosize_x` explicitly opts in.

##### `autosize_x`
Type: `Property`<`Option`<`bool`>>

Optional override for whether autosize manages the `x` axis.

##### `autosize_y`
Type: `Property`<`Option`<`bool`>>

Optional override for whether autosize manages the `y` axis.

##### `corner_radius`
Type: `Property`<`f64`>

Corner radius for the scroller clipping region, in pixels.
When clipping is enabled, pointer targeting follows the same rounded boundary.

##### `snap_positions_x`
Type: `Property`<`Vec`<[`Size`](../../../api/pax-runtime-api/layout.md#size)>>

Scroll snap anchors expressed in px/% along each axis.
Web uses CSS snapping; Apple and Windows use native snapping. Linux preserves
GTK momentum, then snaps after a guarded settling period. Actual native positions
are mirrored back to the bound scroll offsets.

##### `snap_positions_y`
Type: `Property`<`Vec`<[`Size`](../../../api/pax-runtime-api/layout.md#size)>>

Vertical scroll snap anchors expressed in px/%.
