# core::group
<!-- summary: API docs for pax-std::core::group. -->
<!-- tags: api, pax-std -->

## Structs
### `Group`
Gathers a set of children underneath a single render node:
useful for composing transforms and simplifying render trees.

#### Properties
##### `autosize`
Type: `Property`<`bool`>

Automatically sizes the group to its direct content children when possible.

##### `autosize_x`
Type: `Property`<`Option`<`bool`>>

Optional override for whether autosize manages the `x` axis.

##### `autosize_y`
Type: `Property`<`Option`<`bool`>>

Optional override for whether autosize manages the `y` axis.

##### `corner_radius`
Type: `Property`<`f64`>

Corner radius used when the group materializes a native surface, in pixels.
