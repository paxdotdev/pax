# forms::tooltip
<!-- summary: API docs for pax-std::forms::tooltip. -->
<!-- tags: api, pax-std -->

## Structs
### `Tooltip`
A simple hover tooltip that renders slotted content plus a floating text tip.

#### Properties
##### `tip`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<`String`>

Tooltip text.

##### `autosize`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<`bool`>

Automatically sizes the trigger wrapper to its slotted content when possible.

##### `autosize_x`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<`Option`<`bool`>>

Optional override for whether autosize manages the `x` axis.

##### `autosize_y`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<`Option`<`bool`>>

Optional override for whether autosize manages the `y` axis.
