# pax-runtime-api
<!-- summary: Public runtime API types shared across Pax crates, user components, and platform backends. -->
<!-- tags: api, pax-runtime-api -->

Public runtime API types shared across Pax crates, user components, and platform backends.

Most names are reexported at the crate root for compatibility, while their source modules
provide the browsing ontology used by the generated API docs.

## Submodules
- [animation](animation.md)
- [color](color.md)
- [cursor](cursor.md)
- [drawing](drawing.md)
- [events](events.md)
- [layout](layout.md)
- [math](math.md)
- [pax_value](pax_value.md)
- [platform](platform.md)
- [properties](properties.md)
- [rendering](rendering.md)
- [store](store.md)
- [transform](transform.md)
- [unit_value](unit_value.md)
- [variables](variables.md)

## Macros
### `impl_default_coercion_rule`
Implements coercion by converting the selected `PaxValue` variant's contents.
The optional third argument opts into `CoercionRules::is_identity_roundtrip`;
omit it unless the type satisfies that method's exact, side-effect-free contract.
