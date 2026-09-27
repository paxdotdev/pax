# store
<!-- summary: Typed stores owned by mounted nodes and discovered through `NodeContext`. -->
<!-- tags: api, pax-runtime-api -->

Typed stores owned by mounted nodes and discovered through `NodeContext`.

## Enums
### `StoreError`
Failure to register or borrow a store through a `NodeContext`.

#### Variants
##### `Missing` { `type_name`: &'`static` `str` }
No provider of the requested type exists on this node or its logical ancestors.

##### `ExpiredScope`
The context is not mounted, its mount has ended, or its provider ancestry has expired.

##### `BorrowConflict` { `type_name`: &'`static` `str` }
The selected provider is already borrowed by a `with_store` closure.
Lookup does not fall through to an outer provider in this case.

## Traits
### `Store`
Marker trait for values published with `NodeContext::provide_store`.

The concrete Rust type is the lookup key. Each mounted owner can provide one
value per type; a nearer provider of that type shadows an ancestor's value.
Prefer a named struct or newtype for each purpose. A type alias does not
create a distinct key.

A store is ordinary Rust data and does not need `#[pax]`. Use `Property`
handles inside it for reactive state; plain fields do not become reactive.
The owner clears its providers at final unmount. Explicitly cloned handles
can outlive that registration, but do not reconnect to a later mount.
