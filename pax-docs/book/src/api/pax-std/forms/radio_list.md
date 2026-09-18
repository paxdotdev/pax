# forms::radio_list
<!-- summary: API docs for pax-std::forms::radio_list. -->
<!-- tags: api, pax-std -->

## Structs
### `RadioList`
A radio list control, delegating to a platform-specific native control.

#### Properties
##### `background`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<[`Color`](../../../api/pax-runtime-api/color.md#color)>

Radio button background color when unchecked.

##### `background_checked`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<[`Color`](../../../api/pax-runtime-api/color.md#color)>

Radio button background color when checked.

##### `outline`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<[`Stroke`](../../../api/pax-runtime-api/drawing.md#stroke)>

Radio button outline stroke.

##### `options`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<`Vec`<`String`>>

List of selectable option labels.

##### `selected_id`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<`u32`>

Index of the currently selected option.

##### `style`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<[`TextStyle`](../../../api/pax-std/core/text.md#textstyle)>

Text style for option labels.
