# shaders::radiance
<!-- summary: API docs for pax-std::shaders::radiance. -->
<!-- tags: api, pax-std -->

## Structs
### `Radiance`
Experimental point-light projection through a two-color rendered stencil.

Foreground coverage emits the full light field; background-colored coverage
can transmit it and catch a polished sheen. The full-resolution silhouette
and current reveal masks remain crisp. Light color is independent of the
stencil palette. Contour glints are available separately in `RimRefraction`.
This is a stylized 2.5D scattering effect,
not a physically calibrated glass material or arbitrary background refraction.

Enable the `shaders` feature. WGPU captures vector/image children; native
descendants are unsupported. CPU renders the children without the effect.
The caller owns light position, choreography, and visibility.

#### Properties
##### `foreground`
Type: `Property`<[`Color`](../../../api/pax-runtime-api/color.md#color)>

Full-transmission stencil color, white by default. Must match the actual solid fill.

##### `background`
Type: `Property`<[`Color`](../../../api/pax-runtime-api/color.md#color)>

Glass source color, black by default. Must be distinct from foreground.

##### `light_color`
Type: `Property`<[`Color`](../../../api/pax-runtime-api/color.md#color)>

Projected light and sheen color, independent of the stencil colors. Defaults to white.
Alpha scales the light's strength; zero leaves the source unchanged.
CMY tint is controlled separately by the prism controls.

##### `light_x`
Type: `Property`<`f64`>

Light X in container-local logical pixels. Defaults to 820.

##### `light_y`
Type: `Property`<`f64`>

Light Y in container-local logical pixels. Defaults to 185.

##### `light_depth`
Type: `Property`<`f64`>

Positive distance behind the stencil plane, 120–900 local pixels. Defaults to 340.

##### `brightness`
Type: `Property`<`f64`>

Light intensity, 0–6. Defaults to 2.20; zero bypasses rays and glass shading.

##### `reach`
Type: `Property`<`f64`>

Maximum projected distance, 0–360 local pixels. Defaults to 190.

##### `falloff`
Type: `Property`<`f64`>

Distance attenuation exponent, 0.5–4. Defaults to 1.

##### `prism_dispersion`
Type: `Property`<`f64`>

Chromatic separation of the projected light, 0–1. Defaults to 0 (white/tinted light).
Splits the fan into CMY bands around the moving light, up to 36 local pixels apart
on either side. This resamples the captured projection rather than tracing extra rays.

##### `prism_saturation`
Type: `Property`<`f64`>

Saturation of dispersed light, 0–1. Defaults to 0.25. Zero restores neutral light.
Normalizes the overlapping CMY lobes independently of their brightness.

##### `transmission`
Type: `Property`<`f64`>

Fraction transmitted by background-colored coverage, 0–0.5. Defaults to 0.16.
Affects the projected rays and the light visible inside the source; retains source alpha.

##### `polish`
Type: `Property`<`f64`>

Vertical moving sheen strength, 0–1. Defaults to 0.55.
Set this and transmission to zero for solid source coverage.
