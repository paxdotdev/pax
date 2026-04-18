# store
<!-- summary: Store marker traits for runtime-managed state. -->
<!-- tags: api, pax-runtime-api -->

Store marker traits for runtime-managed state.

## Traits
### `Store`
Marker trait for types that can be inserted into a Pax local store.

Stored objects need to be unique for any given stack. Avoid inserting broad
reusable types directly; prefer a local newtype that represents one specific
store purpose.
