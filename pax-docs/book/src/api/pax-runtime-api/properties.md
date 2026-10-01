# properties
<!-- summary: API docs for pax-runtime-api::properties. -->
<!-- tags: api, pax-runtime-api -->

## Traits
### `PropertyBinding`
Owner-thread binding operations used by generated component factories.

Shared fields publish values while their evaluators stay in the entered
[`PropertyGraph`]. Local fields retain ordinary lazy graph semantics.

---

### `PropertyValue`
Value operations shared by local and transferable properties.

Values must be cloneable for `.get()`, interpolatable for transitions, and
`'static` because properties are stored in the runtime graph.
