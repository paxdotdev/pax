# drawing::pax_logo::static_pax_logo
<!-- summary: API docs for pax-std::drawing::pax_logo::static_pax_logo. -->
<!-- tags: api, pax-std -->

## Structs
### `StaticPaxLogo`
The settled Pax logo, using the animation's final geometry without masks.

This component has no playback or click behavior. It uses the same palette
and 920.72 × 463.95 coordinate system as [`AnimatedPaxLogo`].

#### Properties
##### `background_mode`
Type: `Property`<[`LogoBackgroundMode`](../../../../api/pax-std/drawing/pax_logo/animated_pax_logo.md#logobackgroundmode)>

The surrounding surface; Dark exchanges the signboard and letter fills.

##### `fill`
Type: `Property`<[`Paint`](../../../../api/pax-runtime-api/drawing.md#paint)>

Signboard/post fill in Light mode, lettering/counters in Dark mode.

##### `letter_fill`
Type: `Property`<[`Paint`](../../../../api/pax-runtime-api/drawing.md#paint)>

Lettering/backing fill in Light mode, signboard/post in Dark mode.

##### `banner_elements`
Type: `Property`<`Vec`<[`PathElement`](../../../../api/pax-runtime-api/drawing.md#pathelement)>>

##### `p_ring_elements`
Type: `Property`<`Vec`<[`PathElement`](../../../../api/pax-runtime-api/drawing.md#pathelement)>>

##### `p_counter_elements`
Type: `Property`<`Vec`<[`PathElement`](../../../../api/pax-runtime-api/drawing.md#pathelement)>>

##### `a_elements`
Type: `Property`<`Vec`<[`PathElement`](../../../../api/pax-runtime-api/drawing.md#pathelement)>>

##### `a_counter_elements`
Type: `Property`<`Vec`<[`PathElement`](../../../../api/pax-runtime-api/drawing.md#pathelement)>>

##### `x_elements`
Type: `Property`<`Vec`<[`PathElement`](../../../../api/pax-runtime-api/drawing.md#pathelement)>>

##### `roll_start_cap_elements`
Type: `Property`<`Vec`<[`PathElement`](../../../../api/pax-runtime-api/drawing.md#pathelement)>>

##### `roll_body_elements`
Type: `Property`<`Vec`<[`PathElement`](../../../../api/pax-runtime-api/drawing.md#pathelement)>>

##### `roll_end_cap_elements`
Type: `Property`<`Vec`<[`PathElement`](../../../../api/pax-runtime-api/drawing.md#pathelement)>>

##### `top_fabric_elements`
Type: `Property`<`Vec`<[`PathElement`](../../../../api/pax-runtime-api/drawing.md#pathelement)>>

##### `fabric_elements`
Type: `Property`<`Vec`<[`PathElement`](../../../../api/pax-runtime-api/drawing.md#pathelement)>>

##### `backing_fabric_elements`
Type: `Property`<`Vec`<[`PathElement`](../../../../api/pax-runtime-api/drawing.md#pathelement)>>
