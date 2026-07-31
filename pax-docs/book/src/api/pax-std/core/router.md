# core::router
<!-- summary: API docs for pax-std::core::router. -->
<!-- tags: api, pax-std -->

## Structs
### `Route`
Default route branch shell consumed by a parent [`Router`].

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

### `RouteCard`
Declarative route branch presented as a card over the current route.

`RouteCard` is consumed by a parent [`Router`] like [`Route`], but the
presentation behavior is owned by this component shell. It retains the
previously mounted branch underneath so the incoming or outgoing card slides
over stable content while a black scrim fades over the retained branch. The
route's own contents may still declare additional element or component
lifecycle transitions.

#### Properties
##### `path`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<`String`>

Static route pattern, e.g. `/details/:id`.

##### `default`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<`bool`>

Fallback card branch used when no `path` branch matches.

##### `edge`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<[`RouteCardEdge`](/api/pax-std/core/router.md#routecardedge)>

Edge used by the generated card enter/exit transition.

##### `duration`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<[`Duration`](/api/pax-runtime-api/animation.md#duration)>

Duration for the generated card enter/exit transition.

##### `scrim_opacity`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<[`Opacity`](/api/pax-runtime-api/color.md#opacity)>

Maximum opacity for the black scrim over the retained background.
Unitless values are normalized alpha (`0.3` is 30%); percentages such
as `30%` are also supported.

##### `curve`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<[`RouteCardCurve`](/api/pax-std/core/router.md#routecardcurve)>

Reserved easing tuning for the card enter/exit transition.

---

### `RouteModal`
Stacked route branch consumed by a parent [`Router`].

`RouteModal` matches like [`Route`], but it keeps the previously mounted
branch active underneath while the modal branch is active. The shell fades a
black scrim over the retained branch; the route's own contents are
responsible for any modal-specific enter/exit transition.

#### Properties
##### `path`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<`String`>

Static route pattern, e.g. `/tools`.

##### `default`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<`bool`>

Fallback modal branch used when no `path` branch matches.

##### `duration`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<[`Duration`](/api/pax-runtime-api/animation.md#duration)>

Duration for the generated modal scrim enter/exit transition.

##### `scrim_opacity`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<[`Opacity`](/api/pax-runtime-api/color.md#opacity)>

Maximum opacity for the black scrim over the retained background.
Unitless values are normalized alpha (`0.3` is 30%); percentages such
as `30%` are also supported.

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

## Enums
### `RouteCardCurve`
Reserved easing curve values for [`RouteCard`] transition tuning.

#### Variants
##### `Linear`
##### `Hold`
##### `InQuad`
##### `OutQuad`
##### `InOutQuad`
##### `InBack`
##### `OutBack`
##### `InOutBack`
---

### `RouteCardEdge`
Edge used by [`RouteCard`] transitions.

#### Variants
##### `Leading`
##### `Trailing`
##### `Top`
##### `Bottom`
