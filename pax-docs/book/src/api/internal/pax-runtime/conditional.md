# conditional
<!-- summary: API docs for pax-runtime::conditional. -->
<!-- tags: api, pax-runtime -->

## Structs
### `ConditionalInstance`
A special "control-flow" primitive, Conditional (`if`) allows for a
subtree of a component template to be rendered conditionally,
based on the value of the property `boolean_expression`.
The Pax compiler handles ConditionalInstance specially
with the `if` syntax in templates.

---

### `ConditionalProperties`
Contains the expression of a conditional, evaluated as an expression.

#### Properties
##### `boolean_expression`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<`bool`>
