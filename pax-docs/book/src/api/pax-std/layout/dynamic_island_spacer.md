# layout::dynamic_island_spacer
<!-- summary: API docs for pax-std::layout::dynamic_island_spacer. -->
<!-- tags: api, pax-std -->

## Structs
### `DynamicIslandSpacer`
Opt-in space for UIKit's current safe area; does not draw or intercept input.

`<DynamicIslandSpacer />` fills its container's width and measures its height
from the top safe-area inset. Use it in content-sized layout, or bind `inset`
when positioning other content explicitly. `edge` also supports the home
indicator and landscape side insets. Insets update on rotation and window
changes, in debug and release. Other chassis measure zero on the inset axis.

This reserves the platform's entire safe inset, including status bars on
devices without a Dynamic Island. It does not automatically move siblings,
inset ancestors, or detect an ancestor that already avoids the safe area.

#### Properties
##### `edge`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<[`SafeAreaEdge`](../../../api/pax-std/layout/dynamic_island_spacer.md#safeareaedge)>

Window edge to reserve. Defaults to Top. Left/Right measure width and
fill the container's height; Top/Bottom measure height and fill width.

##### `inset`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<`f64`>

Computed inset in logical pixels. Bind to observe it; the spacer owns
this output and overwrites authored values when its measurement changes.

## Enums
### `SafeAreaEdge`
The window safe-area edge reserved by a DynamicIslandSpacer.

#### Variants
##### `Top`
Space below the status bar or top cutout.

##### `Right`
Space left of a right-side cutout in landscape.

##### `Bottom`
Space above the home indicator.

##### `Left`
Space right of a left-side cutout in landscape.
