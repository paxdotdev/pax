# shaders::gaussian_blur
<!-- summary: API docs for pax-std::shaders::gaussian_blur. -->
<!-- tags: api, pax-std -->

## Structs
### `GaussianBlur`
Experimental Gaussian blur of GPU-rendered descendants.

Requires WGPU and captures vectors and renderer-backed images. Native descendants
are unsupported; native overlap masks retain the descendants' original coverage.

#### Properties
##### `radius`
Type: `Property`<`f64`>

Standard deviation in local logical pixels, clamped to 0–48. Defaults to zero (identity).

##### `reference`
Type: `Property`<`bool`>

Force full-resolution separable convolution. Defaults to false, using adaptive resolution.
