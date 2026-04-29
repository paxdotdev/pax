# core::link
<!-- summary: API docs for pax-std::core::link. -->
<!-- tags: api, pax-std -->

## Structs
### `Link`
Navigates to a URL when its slotted content is clicked or tapped.

`Link` remains router-agnostic: it writes a URL, while `Router` and `Route`
declaratively read the current location. On web targets, same-origin
`target=Current` navigation can be serviced through client-side history
updates instead of a full document reload.

#### Properties
##### `url`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<`String`>

Destination URL.

##### `target`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<[`Target`](/api/pax-std/core/link.md#target)>

Whether to open the URL in the current or a new browsing context.

##### `autosize`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<`bool`>

Automatically sizes the link wrapper to its slotted content when possible.

##### `autosize_x`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<`Option`<`bool`>>

Optional override for whether autosize manages the `x` axis.

##### `autosize_y`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<`Option`<`bool`>>

Optional override for whether autosize manages the `y` axis.

## Enums
### `Target`
Navigation target for `Link`.

#### Variants
##### `Current`
Navigate in the current window or tab.

##### `New`
Navigate in a new window or tab.
