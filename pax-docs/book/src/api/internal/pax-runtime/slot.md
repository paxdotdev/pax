# slot
<!-- summary: API docs for pax-runtime::slot. -->
<!-- tags: api, pax-runtime -->

## Structs
### `Slot`
Contains the index value for slot, either a literal or an expression.

#### Properties
##### `index`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<[`Numeric`](/api/pax-runtime-api/pax_value/numeric.md#numeric)>

##### `is_remainder`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<`bool`>

##### `last_node_id`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<`usize`>

##### `showing_node`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<`Weak`<`ExpandedNode`>>

---

### `SlotInstance`
A special "control-flow" primitive (a la `yield` or perhaps `goto`) that
renders projected payload into a node's encapsulated implementation.

`Slot` relies on raw `projected_children` being present on the runtime stack
and will not render any content if there are none. Projection is the engine
transport mechanism; semantic container logic should usually reason in terms
of `received_children` instead.

Consider a Stacker:  the owner of a Stacker passes the Stacker some nodes to render
inside the cells of the Stacker.  To the owner of the Stacker, those nodes might seem like
received children. Inside Stacker's encapsulated implementation, those
received children travel as projected children until `Slot` becomes their
rendered home. This same technique is portable and applicable elsewhere via
`Slot`.
