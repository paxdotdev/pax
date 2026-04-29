# container
<!-- summary: API docs for pax-runtime::container. -->
<!-- tags: api, pax-runtime -->

## Structs
### `ContainerFrame`
Parent-local frame assigned by a container to one of its content children.

This behaves like a virtual wrapper node inside the parent: the frame's
transform is composed onto the parent transform and its bounds become the
child container bounds.

#### Properties
##### `transform`
Type: `Transform2`<[`NodeLocal`](/api/internal/pax-runtime/engine/node_interface.md#nodelocal), [`NodeLocal`](/api/internal/pax-runtime/engine/node_interface.md#nodelocal)>

##### `bounds`
Type: (`f64`, `f64`)

## Enums
### `ContentChildrenSource`
Engine-internal selector for which child family should be normalized into
`NodeContext::content_children`.

#### Variants
##### `Direct`
##### `Slot`
## Traits
### `Container`
Trait for nodes that semantically interpret child content.

Containers can call this from their existing mount logic to install reactive
behavior on top of the runtime's normalized `content_children` view.
