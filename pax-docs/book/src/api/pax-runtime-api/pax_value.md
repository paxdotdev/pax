# pax_value
<!-- summary: API docs for pax-runtime-api::pax_value. -->
<!-- tags: api, pax-runtime-api -->

## Submodules
- [pax_value::functions](pax_value/functions.md)
- [pax_value::numeric](pax_value/numeric.md)

## Enums
### `PaxAny`
This type serves a similar purpose as `Box<dyn Any>`, but allows for special
handling of some types, enabling things like coercion.

#### Variants
##### `Builtin`([`PaxValue`](/api/pax-runtime-api/pax_value.md#paxvalue))
Built-in PAXEL/runtime value.

##### `Any`(`Box`<`dyn` `Any`>)
Arbitrary Rust value boxed for runtime storage.

---

### `PaxValue`
Runtime container for polymorphic values, for evaluating PAXEL.

Two important traits are related to this type:
ToFromPaxValue - responsible for converting to and from specific types (u8,
String, Color, etc)
CoercionRules - responsible for coercing a PaxValue to a specific type
(possibly from multiple different variants)

#### Variants
##### `Bool`(`bool`)
Boolean value.

##### `Numeric`([`Numeric`](/api/pax-runtime-api/pax_value/numeric.md#numeric))
Polymorphic numeric value.

##### `String`(`String`)
UTF-8 string value.

##### `Size`([`Size`](/api/pax-runtime-api/layout.md#size))
Pax size value, such as `25px` or `50%`.

##### `Percent`([`Percent`](/api/pax-runtime-api/color.md#percent))
Raw percent value.

##### `Color`(`Box`<[`Color`](/api/pax-runtime-api/color.md#color)>)
Pax color value.

##### `Rotation`([`Rotation`](/api/pax-runtime-api/transform.md#rotation))
Pax rotation value.

##### `Duration`([`Duration`](/api/pax-runtime-api/animation.md#duration))
Pax animation duration value, such as `250ms`, `1s`, or `10f`.

##### `PathElement`(`Box`<[`PathElement`](/api/pax-runtime-api/drawing.md#pathelement)>)
Vector path element value.

##### `Option`(`Box`<`Option`<[`PaxValue`](/api/pax-runtime-api/pax_value.md#paxvalue)>>)
Optional value.

##### `Vec`(`Vec`<[`PaxValue`](/api/pax-runtime-api/pax_value.md#paxvalue)>)
Homogeneous or heterogeneous vector value.

##### `Range`(`Box`<[`PaxValue`](/api/pax-runtime-api/pax_value.md#paxvalue)>, `Box`<[`PaxValue`](/api/pax-runtime-api/pax_value.md#paxvalue)>)
Range value.

##### `Object`(`Vec`<(`String`, [`PaxValue`](/api/pax-runtime-api/pax_value.md#paxvalue))>)
Object value represented by named fields.

##### `Enum`(`Box`<(`String`, `String`, `Vec`<[`PaxValue`](/api/pax-runtime-api/pax_value.md#paxvalue)>)>)
Enum value represented by type name, variant name, and payload values.

## Traits
### `ImplToFromPaxAny`
Marker trait for types that can be stored inside `PaxAny` without a built-in `PaxValue` representation.

If the type is part of `PaxValue`, implement `CoercionRules` instead.

---

### `ToFromPaxAny`
Trait that marks a type as being representable as a PaxAny, and provides
the implementation for going to/from that type. For all builtins this
means going to/from a pax value. For others to a `Box<dyn Any>`. This
is automatically Implemented for PaxValue types through the macro
impl_to_from_pax_value!, and for other types by implementing the marker
trait ImplToFromPaxAny.

---

### `ToPaxValue`
Converts a Rust value into its built-in `PaxValue` representation.

This is exact conversion, not coercion; coercion is handled separately by
`CoercionRules`.

## Type Aliases
### `RcPaxValue`
Shared runtime PaxValue handle.
