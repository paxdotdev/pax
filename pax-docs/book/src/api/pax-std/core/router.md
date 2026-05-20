# core::router
<!-- summary: API docs for pax-std::core::router. -->
<!-- tags: api, pax-std -->

## Structs
### `Route`
Default route branch shell consumed by a parent [`Router`].

Use `path` for explicit path matching, `:param` for single-segment capture,
and a terminal `*` to consume the remaining tail. Use `default=true` to
provide the fallback branch when no explicit path matches.

`Route` is a normal component shell: `Router` selects and mounts the `Route`
node, and the shell renders its projected children.

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

### `RouteCard`
Declarative route branch presented as a card over the current route.

`RouteCard` is consumed by a parent [`Router`] like [`Route`], but the
presentation behavior lives in the `RouteCard` component shell. It renders its
projected children inside a full-size `Group` with element-level `@in` / `@out`
slide transitions. The route's own contents may still declare additional
element or component lifecycle transitions.

#### Properties
##### `path`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<`String`>

Static route pattern, e.g. `/details/:id`.

##### `default`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<`bool`>

Fallback card branch used when no `path` branch matches.

##### `edge`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<[`RouteCardEdge`](#routecardedge)>

Edge from which the card enters and toward which it exits.

##### `duration`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<[`Duration`](/api/pax-runtime-api/animation.md#duration)>

Duration for the generated card enter/exit transition. Literal time durations
may be written as frames or time units, e.g. `18f` or `240ms`.

##### `curve`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<[`RouteCardCurve`](#routecardcurve)>

Reserved easing tuning for the card transition. The current built-in shell uses
`OutQuad` for its slide.

---

### `Router`
Declarative route reader that selects one active route branch subtree.

`Router` is a control-flow primitive: it does not render by itself.
Instead, it matches the current location against its direct route branch
children and mounts only the winning branch node.

The active subtree receives an implicit `route` binding with:

- `route.location`: the location scoped to this router
- `route.global_location`: the full browser/native location
- `route.params`: named captures from `:param` segments
- `route.remainder`: the unmatched tail after this branch
- `route.is_exact` and `route.consumed_segments`: match metadata

Nested routers match against the nearest ancestor `route.remainder` by
default, which keeps route trees composable without manual string slicing.

#### Custom Route Branches
`Router` does not require branches to be named `Route`. A component or
primitive can act as a route branch when its Rust definition declares the route
branch contract:

```rust
#[pax]
#[route_branch(path = "path", default = "default")]
#[file("src/my_card_route.pax")]
pub struct MyCardRoute {
    pub path: Property<String>,
    pub r#default: Property<bool>,
    pub edge: Property<MyEdge>,
}
```

The `path` and `default` arguments name the component properties that `Router`
reads statically while parsing its direct children. Other properties are owned
by the branch component and remain normal userland configuration. Custom names
are supported:

```rust
#[route_branch(path = "pattern", default = "fallback")]
```

```pax
<Router>
    <MyCardRoute path="/details/:id" edge=MyEdge::Trailing>
        <DetailsPanel />
    </MyCardRoute>
    <MyCardRoute default=true />
</Router>
```

## Enums
### `RouteCardCurve`
Reserved easing curve values for [`RouteCard`] transition tuning.

#### Variants
`Linear`, `Hold`, `InQuad`, `OutQuad`, `InOutQuad`, `InBack`, `OutBack`, `InOutBack`

---

### `RouteCardEdge`
Edge used by [`RouteCard`] transitions.

#### Variants
`Leading`, `Trailing`, `Top`, `Bottom`
