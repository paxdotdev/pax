# forms::combo_box
<!-- summary: API docs for pax-std::forms::combo_box. -->
<!-- tags: api, pax-std -->

## Structs
### `ComboBox`
A text-filtered list control for selecting one item, with optional "new item" behavior.

Clicking or tapping the input opens the list. Filtering is case-sensitive,
with prefix matches first and alphabetical order within each group. Selecting
a row commits its index in `options`, copies its label to `text`, and closes
the list. Editing or blurring the input does not commit a selection or clear
the previous one. Bind `text` for the query and `selected` for the last choice.

This composed control uses a native Textbox and Pax list rows. It does not
implement arrow-key/Enter list selection or a complete accessible combobox
contract.

#### Properties
##### `text`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<`String`>

Text currently shown in the input.

##### `selected`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<`Option`<`usize`>>

Index of the last selected option, or `None`. Editing `text` filters the
list; clicking a row commits its original index in `options`.

##### `options`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<`Vec`<`String`>>

Available option labels; duplicate labels retain distinct indices.
Changing this list refreshes the selected label at the existing index.
An out-of-range selection becomes `None` and clears the text. Reordering
options does not preserve an application's item identity automatically.

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
Show a disabled "No Results Found" row. This is the default.

##### `AllowInvalid`
Allow unmatched text without showing a no-results row. Initial free text
is preserved when `selected` is `None`; this does not commit a new item.

##### `Text`(`String`)
Show custom text when there are no matches; clicking or tapping the row
triggers `@new_item`. The application must add/select the item itself.
