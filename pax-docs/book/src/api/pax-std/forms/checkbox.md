# forms::checkbox
<!-- summary: API docs for pax-std::forms::checkbox. -->
<!-- tags: api, pax-std -->

## Structs
### `Checkbox`
A checkbox control, delegating to a platform-specific native checkbox.

#### Properties
##### `background`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<[`Color`](/api/pax-runtime-api/color.md#color)>

The background color when unchecked

##### `background_checked`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<[`Color`](/api/pax-runtime-api/color.md#color)>

The background color when checked

##### `outline`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<[`Stroke`](/api/pax-runtime-api/drawing.md#stroke)>

The outline stroke of the checkbox

##### `corner_radius`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<`f64`>

The border radius of the checkbox

##### `checked`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<`bool`>

Whether the checkbox is currently checked
