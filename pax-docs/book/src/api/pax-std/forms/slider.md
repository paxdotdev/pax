# forms::slider
<!-- summary: API docs for pax-std::forms::slider. -->
<!-- tags: api, pax-std -->

## Structs
### `Slider`
A slider control, delegating to a platform-specific native range input.

#### Properties
##### `background`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<[`Color`](/api/pax-runtime-api/color.md#color)>

Track background color.

##### `accent`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<[`Color`](/api/pax-runtime-api/color.md#color)>

Accent color for the active track/thumb, when supported.

##### `corner_radius`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<`f64`>

Slider corner radius, in pixels.

##### `value`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<`f64`>

Current slider value.

##### `step`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<`f64`>

Step interval.

##### `min`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<`f64`>

Minimum value.

##### `max`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<`f64`>

Maximum value.
