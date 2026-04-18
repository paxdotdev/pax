# layout::carousel
<!-- summary: API docs for pax-std::layout::carousel. -->
<!-- tags: api, pax-std -->

## Structs
### `Carousel`
A paged scrolling container with native scroll snapping and optional page dots.

Each slotted child becomes one page. The carousel lays pages out along `axis`,
sizes each page with `page_size`, and binds its scroll position through an
internal `Scroller`.

#### Properties
##### `axis`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<[`CarouselAxis`](/api/pax-std/layout/carousel.md#carouselaxis)>

Axis along which pages are laid out and snapped.

##### `page_size`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<[`Size`](/api/pax-runtime-api/layout.md#size)>

Size of each page along the scroll axis (defaults to 100%).

##### `show_dots`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<`bool`>

Whether to show page-position dots when there is more than one page.

##### `scroll_pos_x`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<`f64`>

Horizontal scroll position, in pixels.

##### `scroll_pos_y`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<`f64`>

Vertical scroll position, in pixels.

## Enums
### `CarouselAxis`
Direction for carousel paging and scroll snapping.

#### Variants
##### `Horizontal`
Pages flow left-to-right.

##### `Vertical`
Pages flow top-to-bottom.
