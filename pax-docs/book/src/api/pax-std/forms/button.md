# forms::button
<!-- summary: API docs for pax-std::forms::button. -->
<!-- tags: api, pax-std -->

## Structs
### `Button`
A button control, delegating to a platform-specific native button.

#### Properties
##### `label`
Type: `Property`<`String`>

Text label displayed inside the button.

##### `color`
Type: `Property`<[`Color`](../../../api/pax-runtime-api/color.md#color)>

Button background color.

##### `hover_color`
Type: `Property`<[`Color`](../../../api/pax-runtime-api/color.md#color)>

Button background color while hovered, when supported.

##### `corner_radius`
Type: `Property`<`f64`>

Button corner radius, in pixels.

##### `outline`
Type: `Property`<[`Stroke`](../../../api/pax-runtime-api/drawing.md#stroke)>

Button outline stroke.

##### `style`
Type: `Property`<[`TextStyle`](../../../api/pax-std/core/text.md#textstyle)>

Text style applied to the label.
