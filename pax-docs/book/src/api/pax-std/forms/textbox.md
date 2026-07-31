# forms::textbox
<!-- summary: API docs for pax-std::forms::textbox. -->
<!-- tags: api, pax-std -->

## Structs
### `Textbox`
A text input field, with support for styling and font specification.  Will be composited as a platform-specific
native element, for example an `<input>` element in the browser or a `UITextField` on iOS.

#### Properties
##### `text`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<`String`>

Current text value.

##### `background`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<[`Color`](/api/pax-runtime-api/color.md#color)>

Textbox background color.

##### `placeholder`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<`String`>

Placeholder text shown when empty, when supported by the chassis.

##### `stroke`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<[`Stroke`](/api/pax-runtime-api/drawing.md#stroke)>

Border stroke.

##### `corner_radius`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<`f64`>

Corner radius, in pixels.

##### `style`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<[`TextStyle`](/api/pax-std/core/text.md#textstyle)>

Text style.

##### `outline`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<[`Stroke`](/api/pax-runtime-api/drawing.md#stroke)>

Focus outline stroke.

##### `focus_on_mount`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<`bool`>

Requests focus when the textbox mounts.

##### `multiline`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<`bool`>

Renders the textbox as a multiline text area when supported by the chassis.
