# slot
<!-- summary: API docs for pax-runtime::slot. -->
<!-- tags: api, pax-runtime -->

## Structs
### `Slot`
Contains the index value for slot, either a literal or an expression.

#### Properties
##### `index`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<[`Numeric`](/api/pax-runtime-api/pax_value/numeric.md#numeric)>

##### `last_node_id`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<`usize`>

##### `showing_node`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<`Weak`<`ExpandedNode`>>

---

### `SlotInstance`
A special "control-flow" primitive (a la `yield` or perhaps `goto`) — represents a slot into which
an slot_child can be rendered.  Slot relies on `slot_children` being present
on the runtime stack and will not render any content if there are no `slot_children` found.

Consider a Stacker:  the owner of a Stacker passes the Stacker some nodes to render
inside the cells of the Stacker.  To the owner of the Stacker, those nodes might seem like
"children," but to the Stacker they are "slot_children" — children provided from
the outside.  Inside Stacker's template, there are a number of Slots — this primitive —
that become the final rendered home of those slot_children.  This same technique
is portable and applicable elsewhere via Slot.
