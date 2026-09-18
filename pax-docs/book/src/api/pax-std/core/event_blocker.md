# core::event_blocker
<!-- summary: API docs for pax-std::core::event_blocker. -->
<!-- tags: api, pax-std -->

## Structs
### `EventBlocker`
Native surface that absorbs pointer events before they reach content beneath it.

`background` defaults to transparent. A solid background is useful for modal
underlays that must composite above native controls and scroller canvas islands.

#### Properties
##### `background`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<[`Color`](../../../api/pax-runtime-api/color.md#color)>

Solid background painted by the native surface.
