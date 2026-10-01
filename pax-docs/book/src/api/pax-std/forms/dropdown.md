# forms::dropdown
<!-- summary: API docs for pax-std::forms::dropdown. -->
<!-- tags: api, pax-std -->

## Structs
### `Dropdown`
A dropdown list control, delegating to a platform-specific native dropdown implementation.
Allows the selection of a single option from a list of options.

#### Properties
##### `stroke`
Type: `Property`<[`Stroke`](../../../api/pax-runtime-api/drawing.md#stroke)>

Outline stroke for the dropdown control.

##### `options`
Type: `Property`<`Vec`<`String`>>

List of selectable option labels.

##### `selected_id`
Type: `Property`<`u32`>

Index of the currently selected option.

##### `style`
Type: `Property`<[`TextStyle`](../../../api/pax-std/core/text.md#textstyle)>

Text style for option labels.

##### `background`
Type: `Property`<[`Color`](../../../api/pax-runtime-api/color.md#color)>

Dropdown background color.

##### `corner_radius`
Type: `Property`<`f64`>

Dropdown corner radius, in pixels.
