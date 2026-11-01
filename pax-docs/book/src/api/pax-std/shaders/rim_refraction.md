# shaders::rim_refraction
<!-- summary: API docs for pax-std::shaders::rim_refraction. -->
<!-- tags: api, pax-std -->

## Structs
### `RimRefraction`
Experimental CMY contour glints from a caller-positioned spherical light.

By default, follows the alpha silhouette of GPU-rendered children. For internal
contours in two-color artwork, set `surface_color` and `contrast_color` to its
distinct opaque fills. The original silhouette and alpha remain unchanged.
This is a stylized contour treatment, not refraction of the scene behind it.

Enable the `shaders` feature. WGPU captures vectors and renderer-backed images;
native descendants are unsupported. CPU renders the children without the effect.
The caller owns light position, choreography, and visibility. No Radiance required.

#### Properties
##### `surface_color`
Type: `Property`<`Option`<[`Color`](../../../api/pax-runtime-api/color.md#color)>>

None (default) follows source alpha. Some(color) selects that fill in two-color artwork.

##### `contrast_color`
Type: `Property`<[`Color`](../../../api/pax-runtime-api/color.md#color)>

Unselected fill when surface_color is Some. Defaults to white; must differ from the surface.

##### `light_color`
Type: `Property`<[`Color`](../../../api/pax-runtime-api/color.md#color)>

Neutral highlight color, white by default. Alpha scales the entire rim's strength.

##### `light_x`
Type: `Property`<`f64`>

Light X in container-local logical pixels. Defaults to 820.

##### `light_y`
Type: `Property`<`f64`>

Light Y in container-local logical pixels. Defaults to 185.

##### `light_depth`
Type: `Property`<`f64`>

Positive distance behind the surface, 120–900 local pixels. Defaults to 340.

##### `light_radius`
Type: `Property`<`f64`>

Effective sphere radius, 1–160 local pixels. Defaults to 60; larger values broaden glints.

##### `brightness`
Type: `Property`<`f64`>

Light intensity, 0–6. Defaults to 5; zero bypasses the effect.

##### `intensity`
Type: `Property`<`f64`>

Tone-mapped contour tint strength, 0–1. Defaults to 0.795; zero disables the rim.

##### `rim_width`
Type: `Property`<`f64`>

Inward contour footprint, 0–8 local pixels. Defaults to 3.4; zero disables the rim.

##### `dispersion`
Type: `Property`<`f64`>

Angular CMY separation, 0–1. Defaults to 0.98; one spreads lobes by ±40 degrees.
Zero overlaps all three into a neutral highlight at any saturation.

##### `saturation`
Type: `Property`<`f64`>

Chromatic strength, 0–1. Defaults to 0.77; zero is neutral, one fully chromatic.
Tint is blended over the surface, so white fills do not clip the colors away.
