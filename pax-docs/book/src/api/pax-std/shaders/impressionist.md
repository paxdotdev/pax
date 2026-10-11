# shaders::impressionist
<!-- summary: API docs for pax-std::shaders::impressionist. -->
<!-- tags: api, pax-std -->

## Structs
### `Impressionist`
Experimental painterly filter with deterministic marks anchored to the container.

Requires WGPU and captures vectors and renderer-backed images. Native descendants
are unsupported. Source colors update independently of the marks' breathing motion.

#### Properties
##### `phase`
Type: `Property`<`f64`>

Breathing phase, initially zero. Animate this property to move the marks.

##### `seed`
Type: `Property`<`u32`>

Deterministic mark seed. Defaults to zero.

##### `size`
Type: `Property`<`f64`>

Mark size in local logical pixels, clamped to 4–32. Defaults to zero (effective size 4).
