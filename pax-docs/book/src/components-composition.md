# Components & Composition
<!-- summary: Reusable components, props, and slot patterns. -->
<!-- tags: components, composition -->

- Defining components in Rust and pairing with `.pax` templates.
- Props and public fields: passing data into reusable components.
- Slots and child content: patterns from `slot-particles` and nested layouts.
- Composition of scenes: assembling complex UIs from smaller pieces.
- File organization: keeping component templates discoverable.

For runtime internals and container semantics, see [Runtime Child Ontology](runtime-child-ontology.md). That note defines the distinction between received payload, encapsulated implementation structure, and projection as transport for `slot(...)`.
