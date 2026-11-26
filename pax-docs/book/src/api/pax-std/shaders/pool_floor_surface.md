# shaders::pool_floor_surface
<!-- summary: API docs for pax-std::shaders::pool_floor_surface. -->
<!-- tags: api, pax-std -->

## Structs
### `PoolFloorSurface`
Orthographic water caustics over the projected artwork's paint and alpha.
Phase is integrated wave time. Hold it still to freeze the water while lights move.

#### Properties
##### `phase`
Type: `Property`<`f64`>

Wave time in seconds at normal speed.

##### `density`
Type: `Property`<`f64`>

Spatial frequency, clamped to 0.25–3. Defaults to 1.

##### `amplitude`
Type: `Property`<`f64`>

Wave strength from 0 to 1. Defaults to 0.65.

##### `full_resolution`
Type: `Property`<`bool`>

Render at full physical resolution instead of half on each axis.

##### `identity`
Type: `Property`<`bool`>

Use a pass-through at matching resolution for profiling.

##### `button_rim`
Type: `Property`<`bool`>

Add the scene-lit two-pixel button contour in the same pass.
