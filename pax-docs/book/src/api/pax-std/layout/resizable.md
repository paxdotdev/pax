# layout::resizable
<!-- summary: API docs for pax-std::layout::resizable. -->
<!-- tags: api, pax-std -->

## Structs
### `Resizable`
Divides slotted content into resizable horizontal or vertical sections.

`dividers` contains positions along the main axis. With `n` dividers,
`Resizable` expects `n + 1` slot children.

#### Properties
##### `dividers`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<`Vec`<[`Size`](/api/pax-runtime-api/layout.md#size)>>

Divider positions along the main axis.

##### `direction`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<[`ResizableDirection`](/api/pax-std/layout/resizable.md#resizabledirection)>

Whether sections are split horizontally or vertically.

## Enums
### `ResizableDirection`
Axis direction for a `Resizable` split.

#### Variants
##### `Vertical`
Split content into top-to-bottom sections.

##### `Horizontal`
Split content into left-to-right sections.
