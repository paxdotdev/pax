# forms::color_picker
<!-- summary: API docs for pax-std::forms::color_picker. -->
<!-- tags: api, pax-std -->

## Structs
### `ColorPicker`
Edits a color with a saturation/lightness palette, hue and opacity sliders,
and four numeric RGBA fields. Bind `color` to share edits with the parent.

#### Properties
##### `color`
Type: `Property`<[`Color`](../../../api/pax-runtime-api/color.md#color)>

Selected color. External changes update all controls. Defaults to SLATE.

---

### `ColorPickerSwatch`
A clickable color sample that opens a viewport-anchored `ColorPicker`.
A click or tap outside closes the editor and activates the clicked control.

#### Properties
##### `color`
Type: `Property`<[`Color`](../../../api/pax-runtime-api/color.md#color)>

Selected color shared by the swatch and its editor. Defaults to SLATE.
