# drawing::pax_logo::animated_pax_logo
<!-- summary: API docs for pax-std::drawing::pax_logo::animated_pax_logo. -->
<!-- tags: api, pax-std -->

## Structs
### `AnimatedPaxLogo`
The animated Pax logo, with a normalized playhead and optional click replay.

Bind `progress` to coordinate a slider or a parent-owned timeline. The
component does not start playback on mount or paint its surrounding surface.
Scale an enclosing Group from the original 920.72 × 463.95 coordinate system.

#### Properties
##### `background_mode`
Type: `Property`<[`LogoBackgroundMode`](../../../../api/pax-std/drawing/pax_logo/animated_pax_logo.md#logobackgroundmode)>

Selects the contrasting signboard palette. Defaults to Light to preserve
existing consumers; Dark exchanges `fill` and `letter_fill` throughout.

##### `fill`
Type: `Property`<[`Paint`](../../../../api/pax-runtime-api/drawing.md#paint)>

Signboard/post fill in Light mode, lettering/counters in Dark mode.

##### `letter_fill`
Type: `Property`<[`Paint`](../../../../api/pax-runtime-api/drawing.md#paint)>

Lettering/backing fill in Light mode, signboard/post in Dark mode.

##### `progress`
Type: `Property`<`f64`>

Normalized animation position from the initial pose (0.0) to the
finished logo (1.0). Bind this property for coordinated playback. Clicking
restarts it unless `click_to_replay` is false.

##### `click_to_replay`
Type: `Property`<`bool`>

Restart the 1440ms animation when clicked or tapped. Defaults to true.
Disable for externally driven playback or a noninteractive still logo.

## Enums
### `LogoBackgroundMode`
The surface behind the logo, not the signboard's own color.

#### Variants
##### `Light`
Dark signboard with light letters, for a light surrounding surface.

##### `Dark`
Light signboard with dark letters, for a dark surrounding surface.
