# core::link
<!-- summary: API docs for pax-std::core::link. -->
<!-- tags: api, pax-std -->

## Structs
### `Link`
Navigates to a URL when its slotted content is clicked or tapped.

#### Properties
##### `url`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<`String`>

Destination URL.

##### `target`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<[`Target`](/api/pax-std/core/link.md#target)>

Whether to open the URL in the current or a new browsing context.

## Enums
### `Target`
Navigation target for `Link`.

#### Variants
##### `Current`
Navigate in the current window or tab.

##### `New`
Navigate in a new window or tab.
