# layout::stacker
<!-- summary: API docs for pax-std::layout::stacker. -->
<!-- tags: api, pax-std -->

## Structs
### `ContainerReflowTransition`
Reflow animation applied to surviving children when the stack layout changes.

#### Properties
##### `kind`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<[`ContainerReflowTransitionKind`](../../../api/pax-std/layout/stacker.md#containerreflowtransitionkind)>

Which reflow animation source to use.

##### `frames`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<`u64`>

Duration in frames for `Ease`.

##### `curve`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<[`ContainerReflowCurve`](../../../api/pax-std/layout/stacker.md#containerreflowcurve)>

Curve used for `Ease`.

##### `name`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<`String`>

Reserved for a future named motion-curve lookup when `kind` is `Named`.

---

### `Stacker`
Stacker lays out a series of nodes either
vertically or horizontally (i.e. a single row or column) with a specified gutter in between
each node.  `Stacker`s can be stacked inside of each other, horizontally
and vertically, along with percentage-based positioning and `Transform2D.anchor` to compose any rectilinear 2D layout.

#### Properties
##### `direction`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<[`StackerDirection`](../../../api/pax-std/layout/stacker.md#stackerdirection)>

The direction the stacker should flow its cells

##### `gutter`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<[`Size`](../../../api/pax-runtime-api/layout.md#size)>

Spacing between cells

##### `autosize`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<`bool`>

When true, the stacker uses content-child bounds to autosize its cells
and, when possible, its own bounds as well.

`Stacker` interprets plain `autosize=true` as "autosize the extending
axis only" (`y` for vertical stacks, `x` for horizontal stacks). Use
`autosize_x` / `autosize_y` to override those per-axis defaults.

The underlying shrink-sizing pass only applies when the measured axis
can be resolved without parent-size cycles, so percent-sized children
continue to use the existing top-down layout behavior unless the
corresponding stacker axis is already explicit.

##### `autosize_x`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<`Option`<`bool`>>

Optional override for whether autosize manages the `x` axis.

##### `autosize_y`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<`Option`<`bool`>>

Optional override for whether autosize manages the `y` axis.

##### `sizes`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<`Vec`<`Option`<[`Size`](../../../api/pax-runtime-api/layout.md#size)>>>

Size of each cell, by index.  None-values (or array-index out-of-bounds values)
will fall back to computed, equal-sizing

##### `exit_mode`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<[`ContainerExitMode`](../../../api/pax-std/layout/stacker.md#containerexitmode)>

Whether exiting children stay in normal stack flow or hold their previous frame as ghosts.

##### `reflow_transition`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<[`ContainerReflowTransition`](../../../api/pax-std/layout/stacker.md#containerreflowtransition)>

How surviving children should move when the stack's layout changes.

Defaults to `Snap` so Stackers remain a stable layout primitive unless
reflow motion is explicitly requested.

## Enums
### `ContainerExitMode`
Whether exiting children remain in normal layout flow or become ghosts.

#### Variants
##### `Flow`
Keep exiting children in the stack's in-flow layout until their out-transition finishes.

##### `Ghost`
Hold exiting children at their previous frame as overlays while the remaining children
resolve layout without them.

---

### `ContainerReflowCurve`
Easing curve used by `ContainerReflowTransitionKind::Ease`.

#### Variants
##### `Linear`
##### `Hold`
##### `InQuad`
##### `OutQuad`
##### `InOutQuad`
##### `InBack`
##### `OutBack`
##### `InOutBack`
---

### `ContainerReflowTransitionKind`
Which reflow animation source to use when children move to new stack positions.

#### Variants
##### `Snap`
Snap immediately to the new layout.

##### `Ease`
Use a duration and easing curve.

##### `Named`
Reserved for a future named motion-curve lookup in the current component scope.

---

### `StackerDirection`
Flow direction for a `Stacker`.

#### Variants
##### `Vertical`
Stack children top-to-bottom.

##### `Horizontal`
Stack children left-to-right.
