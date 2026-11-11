# shaders::flashy
<!-- summary: API docs for pax-std::shaders::flashy. -->
<!-- tags: api, pax-std -->

## Structs
### `Flashy`
Experimental, surface-anchored specular glints with soft atmospheric flares.

A seeded field samples the captured children's alpha to place tiny reflectors.
Reflector positions stay fixed. Scoped scene `LightSource`s drive the glints;
move the light to sweep reflections across the artwork. Compose a child vector
with `Material::frosted(...)` for grain and a broad sheen from those same lights.
Optional facet motion changes each core's projected shape and reflection.
Long flare rays share a fixed orientation in the effect's local viewing frame;
soft haze surrounds them. Light/facet motion changes intensity, not ray direction.
Flares can extend beyond the source silhouette without replacing the artwork.
This is a stylized sequin treatment, not a physical material or particle simulation.

Enable the `shaders` feature. WGPU captures vectors and renderer-backed images;
native Text and controls cannot be captured. Use vector outlines for lettering.
CPU renders the children unchanged. The caller owns motion and visibility.

#### Properties
##### `phase`
Type: `Property`<`f64`>

Optional facet-motion position in turns, initially zero. One turn loops seamlessly.
Leave fixed for a stationary surface and animate the light position instead.

##### `seed`
Type: `Property`<`u32`>

Deterministic spatial distribution and reflector orientations. Defaults to 17.

##### `spacing`
Type: `Property`<`f64`>

Reflector spacing in local logical pixels, clamped to 8–48. Defaults to 18.
Larger values create fewer, larger glints. Changing spacing redistributes them.

##### `brightness`
Type: `Property`<`f64`>

Glint strength, clamped to 0–4. Defaults to 1.4; zero preserves the captured material.
Scene light intensity controls the shared illumination of materials and glints.

##### `flare`
Type: `Property`<`f64`>

Atmospheric flare strength and spread, clamped to 0–5. Defaults to 0.65.
Zero retains pinpoint cores. Up to 1 increases strength and spread;
above 1 scales ray length and haze radius, reaching five times the size at 5.
Maximum support is 4.75 times spacing; larger flares cost more GPU work.
