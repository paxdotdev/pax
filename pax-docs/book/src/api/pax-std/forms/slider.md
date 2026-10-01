# forms::slider
<!-- summary: API docs for pax-std::forms::slider. -->
<!-- tags: api, pax-std -->

## Structs
### `Slider`
A slider control, delegating to a platform-specific native range input.

#### Properties
##### `background`
Type: `Property`<[`Color`](../../../api/pax-runtime-api/color.md#color)>

Track background color.

##### `accent`
Type: `Property`<[`Color`](../../../api/pax-runtime-api/color.md#color)>

Accent color for the active track/thumb, when supported.

##### `corner_radius`
Type: `Property`<`f64`>

Slider corner radius, in pixels.

##### `value`
Type: `Property`<`f64`>

Current slider value.

##### `step`
Type: `Property`<`f64`>

Step interval.

##### `min`
Type: `Property`<`f64`>

Minimum value.

##### `max`
Type: `Property`<`f64`>

Maximum value.
