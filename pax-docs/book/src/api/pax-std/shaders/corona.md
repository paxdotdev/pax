# shaders::corona
<!-- summary: API docs for pax-std::shaders::corona. -->
<!-- tags: api, pax-std -->

## Structs
### `Corona`
Experimental, colorable corona around a two-color GPU-rendered stencil.

Supply the distinct, opaque foreground/background colors actually used by the
source. Foreground coverage emits independently of its luminance, including
internal boundaries. Transparent regions, source opacity and masks are supported;
arbitrary gradients and multicolor sources are not. The core keeps its geometry.

Enable the `shaders` Cargo feature. WGPU captures vectors and renderer-backed
images; native descendants are unsupported. Vello CPU renders the children
without the effect. Motion is caller-owned: a fixed phase freezes the field.

#### Properties
##### `foreground`
Type: `Property`<[`Color`](../../../api/pax-runtime-api/color.md#color)>

Emitting source color. Defaults to white; must match the source's actual fill.

##### `background`
Type: `Property`<[`Color`](../../../api/pax-runtime-api/color.md#color)>

Non-emitting source color. Defaults to black; must differ from foreground.

##### `tint`
Type: `Property`<`Option`<[`Color`](../../../api/pax-runtime-api/color.md#color)>>

Optional emission tint. None follows foreground, including dark-on-light palettes.
Tint changes only the emitted light; its alpha scales emission coverage.

##### `phase`
Type: `Property`<`f64`>

Periodic motion position in cycles, initially zero. The caller owns time and visibility.

##### `emission`
Type: `Property`<`f64`>

Emission-layer opacity, clamped to 0–1. Defaults to 1; zero leaves surface shading.

##### `reach`
Type: `Property`<`f64`>

Maximum projection distance in container-local logical pixels, 0–120. Defaults to 20.

##### `amplitude`
Type: `Property`<`f64`>

Variation between short and long flares, 0–1. Defaults to 0.8.

##### `falloff`
Type: `Property`<`f64`>

Distance attenuation exponent, 0.5–5. Defaults to 2.7.

##### `surface`
Type: `Property`<`f64`>

Surface tonal depth, 0–0.35. Defaults to 0.04; zero retains source colors.
