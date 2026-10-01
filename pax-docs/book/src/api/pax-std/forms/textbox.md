# forms::textbox
<!-- summary: API docs for pax-std::forms::textbox. -->
<!-- tags: api, pax-std -->

## Structs
### `Textbox`
A text input field, with support for styling and font specification.  Will be composited as a platform-specific
native element, for example an `<input>` element in the browser or a `UITextField` on iOS.

#### Properties
##### `text`
Type: `Property`<`String`>

Current text value.

##### `background`
Type: `Property`<[`Color`](../../../api/pax-runtime-api/color.md#color)>

Textbox background color.

##### `placeholder`
Type: `Property`<`String`>

Placeholder text shown when empty, when supported by the chassis.

##### `stroke`
Type: `Property`<[`Stroke`](../../../api/pax-runtime-api/drawing.md#stroke)>

Border stroke.

##### `corner_radius`
Type: `Property`<`f64`>

Corner radius, in pixels.

##### `style`
Type: `Property`<[`TextStyle`](../../../api/pax-std/core/text.md#textstyle)>

Text style.

##### `outline`
Type: `Property`<[`Stroke`](../../../api/pax-runtime-api/drawing.md#stroke)>

Focus outline stroke.

##### `focus_on_mount`
Type: `Property`<`bool`>

Requests focus when the textbox mounts.

##### `multiline`
Type: `Property`<`bool`>

Renders the textbox as a multiline text area when supported by the chassis.
