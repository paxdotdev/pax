# forms::combo_box
<!-- summary: API docs for pax-std::forms::combo_box. -->
<!-- tags: api, pax-std -->

## Structs
### `ComboBox`
A text-filtered list control for selecting one item, with optional "new item" behavior.

#### Properties
##### `text`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<`String`>

Text currently shown in the input.

##### `selected`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<`Option`<`usize`>>

Selected option index, or `None` when there is no valid selection.

##### `options`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<`Vec`<`String`>>

Available option labels.

##### `new_item`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<[`NewItem`](../../../api/pax-std/forms/combo_box.md#newitem)>

Behavior when the typed text does not match any option.

##### `background`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<[`Color`](../../../api/pax-runtime-api/color.md#color)>

Textbox/list background color.

##### `stroke`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<[`Stroke`](../../../api/pax-runtime-api/drawing.md#stroke)>

Textbox/list stroke.

##### `style`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<[`TextStyle`](../../../api/pax-std/core/text.md#textstyle)>

Text style for the textbox and list items.

##### `corner_radius`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<`f64`>

Textbox corner radius, in pixels.

## Enums
### `NewItem`
Behavior when typed combo-box text does not match an existing option.

#### Variants
##### `Disallow`
Show "No items found" and do not allow adding a new item.

##### `AllowInvalid`
Allows invalid text in the text box on commit, setting `selected` to `None`.

##### `Text`(`String`)
Shows custom text when there are no matches; clicking it triggers the `@new_item` event.
