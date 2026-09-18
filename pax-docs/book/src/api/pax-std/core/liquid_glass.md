# core::liquid_glass
<!-- summary: API docs for pax-std::core::liquid_glass. -->
<!-- tags: api, pax-std -->

## Structs
### `LiquidGlass`
Applies an Apple liquid-glass native effect to supported descendant native surfaces.

#### Properties
##### `enabled`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<`bool`>

Whether this liquid-glass scope is active. When false, descendants opt out.

##### `spacing`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<[`Size`](../../../api/pax-runtime-api/layout.md#size)>

Desired spacing between grouped glass surfaces, in Pax units.

##### `interactive`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<`bool`>

Whether supported Apple glass surfaces should use the interactive effect.

##### `tint`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<`Option`<[`Color`](../../../api/pax-runtime-api/color.md#color)>>

Optional tint for supported Apple glass surfaces.

##### `variant`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<`String`>

Apple glass style. Supported values are currently "regular" and "clear".
