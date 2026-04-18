# layout::stacker
<!-- summary: API docs for pax-std::layout::stacker. -->
<!-- tags: api, pax-std -->

## Structs
### `Stacker`
Stacker lays out a series of nodes either
vertically or horizontally (i.e. a single row or column) with a specified gutter in between
each node.  `Stacker`s can be stacked inside of each other, horizontally
and vertically, along with percentage-based positioning and `Transform2D.anchor` to compose any rectilinear 2D layout.

#### Properties
##### `direction`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<[`StackerDirection`](/api/pax-std/layout/stacker.md#stackerdirection)>

The direction the stacker should flow its cells

##### `gutter`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<[`Size`](/api/pax-runtime-api/layout.md#size)>

Spacing between cells

##### `sizes`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<`Vec`<`Option`<[`Size`](/api/pax-runtime-api/layout.md#size)>>>

Size of each cell, by index.  None-values (or array-index out-of-bounds values)
will fall back to computed, equal-sizing

## Enums
### `StackerDirection`
Flow direction for a `Stacker`.

#### Variants
##### `Vertical`
Stack children top-to-bottom.

##### `Horizontal`
Stack children left-to-right.
