# shaders::cloud_glass
<!-- summary: API docs for pax-std::shaders::cloud_glass. -->
<!-- tags: api, pax-std -->

## Structs
### `CloudGlass`
Shallow cloud glass clipped to the alpha of the projected face.
Keep text, icons, and hit targets outside the effect.

#### Properties
##### `density`
Type: `Property`<`f64`>

Amount of gas (0–8). Higher values grow the wisps and populate more pockets.

##### `gas_color`
Type: `Property`<[`Color`](../../../api/pax-runtime-api/color.md#color)>

Scattered-light color of the gas, independent of the clear shell.

##### `gas_opacity`
Type: `Property`<`f64`>

Opacity of occupied gas (0–1). The wisp shapes and shell lighting stay fixed.

##### `billow_scale`
Type: `Property`<`f64`>

Characteristic radius of a billow in local logical pixels.

##### `button_rim`
Type: `Property`<`bool`>

Add the scene-lit CMY button glint across the 2 px shell bevel. Defaults to false.

##### `phase`
Type: `Property`<`f64`>

Integrated drift time, in seconds at normal speed. Hold to freeze.
