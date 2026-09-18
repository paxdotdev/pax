# repeat
<!-- summary: API docs for pax-runtime::repeat. -->
<!-- tags: api, pax-runtime -->

## Structs
### `RepeatInstance`
A special "control-flow" primitive associated with the `for` statement.
Repeat allows for nodes to be rendered dynamically per data specified in `source_expression`.
That is: for a `source_expression` of length `n`, `Repeat` will render its
template `n` times, each with an embedded component context (`RepeatItem`)
with an index `i` and a pointer to that relevant datum `source_expression[i]`

#### Properties
##### `base`
Type: [`BaseInstance`](../../../api/internal/pax-runtime/rendering.md#baseinstance)

---

### `RepeatItem`
Per-iteration bindings exposed inside a `for` template body.

#### Properties
##### `elem`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<[`PaxValue`](../../../api/pax-runtime-api/pax_value.md#paxvalue)>

##### `i`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<`usize`>

---

### `RepeatProperties`
Contains modal _vec_ and _range_ variants, describing whether the Repeat source
is encoded as a `Vec<T>` (where T is a `PaxValue` properties type) or as a `Range<isize>`

#### Properties
##### `source_expression`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<[`PaxValue`](../../../api/pax-runtime-api/pax_value.md#paxvalue)>

##### `iterator_i_symbol`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<`Option`<`String`>>

##### `iterator_elem_symbol`
Type: [`Property`](../../../api/pax-runtime-api/properties.md#property)<`Option`<`String`>>

##### `repeat_key_expression`
Type: `Option`<`ExpressionInfo`>
