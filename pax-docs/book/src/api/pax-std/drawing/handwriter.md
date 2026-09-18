# drawing::handwriter
<!-- summary: API docs for pax-std::drawing::handwriter. -->
<!-- tags: api, pax-std -->

## Structs
### `Handwriter`
Renders text as single-stroke vector paths suitable for handwriting effects.

`Handwriter` uses bundled SVG stroke fonts and translates text into
`PathElement` data for an inner `Path`. Use `draw_start` and `draw_end` to
animate the visible writing range.

#### Properties
##### `text`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<`String`>

Text to render. Newline characters create additional baselines.

##### `font`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<[`HandwriterFont`](../../../api/pax-std/drawing/handwriter.md#handwriterfont)>

Bundled stroke font used to draw `text`.

##### `stroke`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<[`Stroke`](../../../api/pax-runtime-api/drawing.md#stroke)>

Stroke used for the generated path.

##### `smoothing`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<[`PathSmoothing`](../../../api/pax-runtime-api/drawing.md#pathsmoothing)>

Optional curve smoothing applied to generated path geometry.

##### `alt_text`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<`String`>

Accessible text label. When empty, `text` is used.

##### `selectable`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<`bool`>

Whether the invisible native text layer can be selected.

##### `draw_start`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<[`UnitValue`](../../../api/pax-runtime-api/unit_value.md#unitvalue)>

Start position of the visible handwriting range.

##### `draw_end`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<[`UnitValue`](../../../api/pax-runtime-api/unit_value.md#unitvalue)>

End position of the visible handwriting range.

##### `line_height`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<`f64`>

Baseline-to-baseline multiplier relative to the font's em size.

## Enums
### `HandwriterFont`
Bundled single-stroke fonts available to `Handwriter`.

#### Variants
##### `EMSAllure`
Cursive, airy script.

##### `EMSDelight`
Friendly rounded print hand.

##### `EMSInvite`
Tall invitation script.

##### `EMSLeague`
Flowing connected script.

##### `EMSNeato`
Casual narrow script.

##### `EMSOsmotron`
Rectilinear plotter hand.

##### `EMSReadability`
Highly readable manuscript hand.

##### `EMSTech`
Technical drafting hand.

##### `HersheySans1`
Classic Hershey sans stroke font.

##### `HersheyScript1`
Classic Hershey connected script.
