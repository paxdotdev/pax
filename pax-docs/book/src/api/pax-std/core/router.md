# core::router
<!-- summary: API docs for pax-std::core::router. -->
<!-- tags: api, pax-std -->

## Structs
### `Route`
Declarative route branch consumed by a parent [`Router`].

Use `path` for explicit path matching, `:param` for single-segment capture,
and a terminal `*` to consume the remaining tail. Use `default=true` to
provide the fallback branch when no explicit path matches.

#### Properties
##### `path`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<`String`>

Static route pattern, e.g. `/docs/:slug` or `/settings/*`.

Patterns are matched against the current router scope, not always the
full global path. At the root router, that scope is the full location.

##### `default`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<`bool`>

Fallback branch used when no `path` branch matches.

---

### `Router`
Declarative route reader that selects one active [`Route`] child subtree.

`Router` is a control-flow primitive: it does not render by itself.
Instead, it matches the current location against its child `Route` branches
and mounts only the winning subtree.

The active subtree receives an implicit `route` binding with:

- `route.location`: the location scoped to this router
- `route.global_location`: the full browser/native location
- `route.params`: named captures from `:param` segments
- `route.remainder`: the unmatched tail after this branch
- `route.is_exact` and `route.consumed_segments`: match metadata

Nested routers match against the nearest ancestor `route.remainder` by
default, which keeps route trees composable without manual string slicing.
