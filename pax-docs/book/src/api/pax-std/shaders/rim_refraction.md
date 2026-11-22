# shaders::rim_refraction
<!-- summary: API docs for pax-std::shaders::rim_refraction. -->
<!-- tags: api, pax-std -->

## Structs
### `RimRefraction`
Experimental CMY contour glints from a caller-positioned sphere or scoped scene lights.

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

Neutral highlight color, white by default. Alpha sets the scene-lit rim's
final blend opacity. In sphere mode it scales light strength before tone mapping.

##### `scene_lighting`
Type: `Property`<`bool`>

Use the container's scoped direct lights and camera. Defaults to false (caller-positioned sphere).

##### `light_threshold`
Type: `Property`<`f64`>

Scene mode starts revealing the rim at this direct specular response, reaching full activation
0.08 above it. Defaults to 0.30. Ambient light does not enter the threshold.

##### `light_span`
Type: `Property`<`f64`>

Scene mode: full falloff span along the contour for each finite emitter, as a
fraction of the container width. Zero (default) leaves its response unfocused.
Point and strip contributions add before thresholding; directional lights are unchanged.

##### `hue_cycles`
Type: `Property`<`f64`>

CMY cycles per visible, focused point or strip glint (0.25–8, default 1).
One cycle shows one cyan, magenta, and yellow band without changing the lit footprint.

##### `hue_travel`
Type: `Property`<`f64`>

Focused-glint hue turns per surface width of rightward or downward travel (−8–8).
Defaults to 0 (fixed band order). Negative values reverse the rotation.

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

Caller-positioned sphere intensity, 0–6. Defaults to 5; zero bypasses the effect.

##### `intensity`
Type: `Property`<`f64`>

Tone-mapped contour tint strength, 0–1. Defaults to 0.795; zero disables the rim.

##### `rim_width`
Type: `Property`<`f64`>

Inward contour footprint, 0–8 local pixels. Defaults to 3.4; zero disables the rim.

##### `dispersion`
Type: `Property`<`f64`>

Angular CMY separation, 0–1. Defaults to 0.98; one spreads the sphere-mode lobes by ±40 degrees.
Zero overlaps all three into a neutral highlight at any saturation.

##### `saturation`
Type: `Property`<`f64`>

Chromatic strength, 0–1. Defaults to 0.77; zero is neutral, one fully chromatic.
Tint is blended over the surface, so white fills do not clip the colors away.
