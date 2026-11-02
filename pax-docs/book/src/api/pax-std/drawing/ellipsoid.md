# drawing::ellipsoid
<!-- summary: API docs for pax-std::drawing::ellipsoid. -->
<!-- tags: api, pax-std -->

## Structs
### `Ellipsoid`
An opaque ellipsoid with width × height × depth extents, centered on local Z=0.
Requires `pax-kit/3d` and the GPU renderer. Ordinary transforms, hard clips,
lighting, and depth-aware native-control occlusion apply to the complete volume.
Root `@camera` pan, zoom, rotation and perspective apply to rendering, picking
and occlusion. Near/far clipping cuts the surface without adding caps.

Node and ancestor opacity fade the assembled opaque appearance through the
bounded spatial group compositor; hidden rear surfaces stay hidden.
This primitive accepts a solid opaque color. Translucent material color
and alpha-masked solid content are unsupported;
unsupported instances are omitted with a diagnostic. Zero/negative dimensions
or a singular pose produce no coverage. Transparent shells are a separate contract.
An ellipsoid can supply a detached alpha-mask source; its projected silhouette,
hard clips and assembly opacity contribute to the mask's alpha.
With `pax-kit/shaders`, shader containers resolve internal mesh depth before
filtering. Their output is a flat image at the container's plane; logical
input and native coverage continue to follow the original geometry.

#### Properties
##### `depth`
Type: `Property`<[`Depth`](../../../api/pax-runtime-api/drawing.md#depth)>

Absolute local Z extent; defaults to 100 logical pixels.

##### `color`
Type: `Property`<[`Color`](../../../api/pax-runtime-api/color.md#color)>

Opaque surface color. Defaults to white.

##### `material`
Type: `Property`<[`Material`](../../../api/pax-runtime-api/drawing.md#material)>

Surface response to ordinary Pax scene lights.
