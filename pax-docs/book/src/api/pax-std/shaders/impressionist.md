# shaders::impressionist
<!-- summary: API docs for pax-std::shaders::impressionist. -->
<!-- tags: api, pax-std -->

## Structs
### `Impressionist`
Experimental painterly filter with deterministic marks anchored to the container.

Requires WGPU and captures vectors and renderer-backed images. Native descendants
are unsupported. Source colors update independently of the procedural brush motion.

#### Properties
##### `mode`
Type: `Property`<[`ImpressionistMode`](../../../api/pax-std/shaders/impressionist.md#impressionistmode)>

Brush treatment. Defaults to `ImpressionistMode::Basic`.

##### `phase`
Type: `Property`<`f64`>

Motion position, initially zero. Drives mark drift, breathing, and VanGogh's evolving swirls.
Source content can animate independently; a fixed phase freezes the procedural motion.
Advance at a chosen rate for speed control, or bind a timeline to scrub/ease/reverse motion.

##### `seed`
Type: `Property`<`u32`>

Deterministic mark seed. Defaults to zero.

##### `size`
Type: `Property`<`f64`>

Brush scale in local logical pixels, clamped to 4–32. Each mode applies its own mark proportions.
Also scales directional patterns, including VanGogh's whorls. Defaults to zero (effective size 4).

## Enums
### `ImpressionistMode`
Procedural brush treatments inspired by different painting techniques.

These change mark construction and pigment handling, not scene composition.
All modes retain seeded anchors on the surface; marks move around those anchors with `phase`.

#### Variants
##### `Basic`
The original three scales of softly layered, oval brush marks.

##### `Gauguin`
Broad directional sweeps, simplified color planes, and quiet bristle texture.

##### `VanGogh`
Phase-driven evolving flows with pronounced light and shadow along paint ridges.

##### `Signac`
Closely packed rectangular dabs with interwoven pale and chromatic pigment.

##### `Cezanne`
Varied patches of parallel marks and nested arcs, with broken, wispy contours.

##### `Seurat`
Fine, dense, rounded points of color with restrained pale pigment.
