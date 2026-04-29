# Router (Draft)
<!-- summary: Draft proposal for declarative, cross-platform routing in Pax. -->
<!-- tags: router, navigation, state, templates -->

## Problem

Pax can already navigate to URLs imperatively through `NodeContext::navigate_to(...)`, and `Link` can open a URL when clicked. What it does not have yet is a declarative routing model that fits Pax's template semantics:

- route selection should be expressible in Pax, not only in Rust
- route matching should compose across component boundaries
- browser URLs should integrate naturally on web targets
- native targets should still have a first-class route state even without a browser
- route reads should be declarative, while route writes stay imperative

The router should therefore behave less like a web-only convenience API and more like a Pax-native state machine that happens to serialize cleanly to browser URLs.

## Goals

- Add a declarative way to map route state to rendered subtrees.
- Keep the canonical route state cross-platform and structured, not just a raw URL string.
- Make nested routers compose naturally across component boundaries.
- Avoid new grammar where ordinary primitive/property syntax is sufficient.
- Expose matched params ergonomically inside the active route subtree through a default binding.
- Keep mutation of route state out of templates; writes should come from Rust or platform events.

## Non-goals

- A full web-framework-style loader/action/data-fetching system.
- Regex routes, arbitrary path grammars, or a CSS-sized matching language.
- Typed path-param coercion in the first pass.
- Query-string pattern matching in the first pass.
- Adding router-specific surface area to `Link` in the first phase.

## Existing Surface

Today Pax already has two adjacent primitives:

- `Link` for direct URL navigation from templates
- `NodeContext::navigate_to(...)` for imperative URL navigation from Rust

Those are useful, but they operate on URLs directly and do not define a shared route-state model. The router should sit one layer above them: a shared location store plus declarative readers over that store.

## Alternatives Considered

### 1. Raw URL string as the canonical internal state

This is attractive because browser integration is immediate. It is also the wrong abstraction for Pax's longer-term cross-platform model:

- every reader has to reparse the string
- query/path/fragment structure is not explicit
- non-web targets inherit a browser-shaped API as their only representation
- nested routers become string-slicing exercises

Recommendation: use a structured route state internally, and treat the URL as its web serialization.

### 2. Param declarations embedded in the route string, e.g. `foo/bar/{baz}`

This collides with Pax's existing use of `{...}` for PAXEL expressions and creates a special parsing rule inside string literals. It is clever, but it is not clean.

Recommendation: do not use `{param}` inside route strings.

### 3. Captured params introduced as direct local identifiers

Example:

```pax
<Route path="/docs/:slug">
    <DocPage slug={slug} />
</Route>
```

This looks compact, but it creates several problems:

- name collisions become easy
- it introduces another special scoped-binding rule
- later additions like query, fragment, remainder, and match metadata have nowhere obvious to live

Recommendation: bind one match object, then read params from that object.

### 4. Separate route match modes like `Exact` vs `Prefix`

An explicit match mode is workable, but it adds API surface for something that mainstream routers usually spell in the path pattern itself via catch-all or splat segments.

Recommendation: prefer a terminal `*` in the path over a separate `mode` property for the first pass.

## Proposal

### Canonical Route State

The runtime should own one canonical `RouteLocation` value, conceptually:

```rust
pub struct RouteLocation {
    pub path_segments: Vec<String>,
    pub query: HashMap<String, Vec<String>>,
    pub fragment: Option<String>,
}
```

Notes:

- `path_segments` is the canonical path representation
- query values should preserve repeated keys
- the browser URL is the web serialization of this structure, not the structure itself
- native targets can use the same route state without pretending to be a browser

Normalization rules for the first implementation should stay simple:

- root is `/`
- authored route patterns may include a leading slash, but nested routers may also use relative forms like `settings/*`
- trailing slash differences should normalize away except for root
- matching operates on decoded path segments

### Authoring Surface

The authoring model should use two non-rendering primitives:

- `Router`
- `Route`

Example:

```pax
<Router>
    <Route path="/">
        <HomePage />
    </Route>

    <Route path="/docs/:slug">
        <DocPage slug={route.params.slug} />
    </Route>

    <Route path="/settings/*">
        <SettingsShell />
    </Route>

    <Route default=true>
        <NotFoundPage />
    </Route>
</Router>
```

Each matched `Route` introduces a default binding named `route` into its active subtree:

```pax
<Route path="/users/:user_id">
    <UserPage user_id={route.params.user_id} />
</Route>
```

For the first phase, that binding name should not be configurable. A default binding keeps the authoring surface smaller and still gives the router a place to grow:

- `route.params.user_id`
- `route.location.query`
- `route.location.fragment`
- `route.global_location`
- `route.remainder`

This is a better fit than inventing one local identifier per captured segment, and simpler than exposing an `as=...` option before there is evidence that custom names are actually needed.

### Match Semantics

Within one `Router`:

- `Route` children are evaluated in template order
- the first matching route wins
- `default=true` provides fallback behavior
- if nothing matches and no default route exists, the router renders nothing
- `default=true` is the recommended first spelling because Pax template attributes are currently key/value pairs; if Pax later adopts valueless boolean attributes generally, this can collapse naturally to `default`

`Route.path` in the first phase should support:

- literal segments: `/settings/profile`
- captured segments: `/docs/:slug`
- a terminal catch-all `*`: `/settings/*`

It should not yet support:

- regex fragments
- query matching in the pattern string
- wildcard forms other than terminal `*`

Terminal `*` should match zero or more remaining path segments. That gives nested routing the needed prefix behavior without a separate `mode` property:

- `/settings/*` matches `/settings`
- `/settings/*` matches `/settings/team`
- `/settings/*` matches `/settings/team/42`

This follows the broad direction of routing prior art: React Router uses terminal splats like `/*`, while SvelteKit, Next.js, and Vue Router all expose explicit catch-all/rest route forms rather than a separate "prefix mode" toggle.

### Nested Routers

Nested routers should compose by default across component boundaries.

Example:

```pax
<Router>
    <Route path="/settings/*">
        <SettingsShell />
    </Route>
</Router>
```

```pax
<Router>
    <Route path="/">
        <SettingsHome />
    </Route>

    <Route path="/team/:team_id">
        <TeamSettings team_id={route.params.team_id} />
    </Route>
</Router>
```

If the global route is `/settings/team/42`, the outer route matches `/settings`, and the inner router should see `/team/42` as its input.

This suggests the following rule:

- all routers observe one global `RouteLocation`
- a router without a matched ancestor route evaluates against the full path
- a router inside a matched `Route` evaluates against that route's remainder by default

This is the right default for composition. Making every router match the full global path by default would force nested components to repeat ancestor prefixes and would make routerized components much less portable.

To avoid hiding the true browser/native location from nested consumers, the match object should expose both:

- `route.location`: the scoped location that this router actually matched against
- `route.global_location`: the full global location before ancestor remainder trimming

That keeps the nested-router golden path simple while still allowing deeper components to inspect or reason about the full route when needed. The naming is intentional:

- `route.location` is the opinionated default because most route logic should operate on the scoped location the current router is responsible for
- `route.global_location` is the escape hatch when a nested consumer needs the untrimmed app location

### Match Object Shape

The active route binding should expose a structured `RouteMatch`, conceptually:

```rust
pub struct RouteMatch {
    pub location: RouteLocation,
    pub global_location: RouteLocation,
    pub params: HashMap<String, String>,
    pub consumed_segments: usize,
    pub remainder: Vec<String>,
    pub is_exact: bool,
}
```

For PAXEL ergonomics, `params` should be accessible by identifier-like field name when possible:

```pax
route.params.slug
```

Bracket access can remain the fallback for any future dynamic lookups.

The terminal `*` should not manufacture a magic `"*"` param in the first pass. The unmatched tail is already represented more clearly by `route.remainder`, and nested routers consume that same remainder directly.

### Query and Fragment

The router should carry query and fragment data in `RouteLocation`, but first-phase branch selection should remain path-driven.

That means:

- query and fragment are readable from `route.location` and `route.global_location`
- authors can branch on them with ordinary `if` expressions if needed
- route pattern syntax does not need to solve query matching yet

This keeps the first routing surface small while still preserving browser URL fidelity.

### Write Semantics

Routers should not mutate route state themselves.

The model should be:

- templates read route state declaratively
- platform events can update route state
- Rust code can update route state imperatively

This aligns with the ticket's intuition that no one router should have privileged write access.

The follow-up imperative API can be designed separately, but it should conceptually operate on the structured route state, not only on raw URL strings.

### Web Integration and `Link`

`Link` should stay router-agnostic in the first phase. The router does not need its own special link primitive yet.

On web targets, the chassis should instead own client-side route synchronization:

- initialize the route store from `window.location`
- listen for `popstate`
- update browser history with `pushState` / `replaceState` for same-origin route changes in the current tab
- pipe those history changes back through the chassis/runtime route store

That gives Pax JS-driven routing behavior without forcing nested URI changes to trigger full document reloads. External URLs and `target=Target::New` should continue to behave like ordinary browser navigation.

## Compiler and Runtime Implications

This proposal implies real compiler/runtime work later, even though this ticket is spec-only:

- `Router` and `Route` should be first-class non-rendering primitives
- the compiler/runtime boundary will need a serialized representation for route nodes, including terminal-catch-all semantics
- the runtime needs a shared route store and a matcher
- web chassis support needs URL/history synchronization
- the implicit `route` binding needs to be visible to expressions inside the matched subtree

This does not look like a good fit for a pure userland component layered only on today's primitives. The scoping and remainder behavior justify making routing a first-class control-flow feature.

## Recommended First Implementation Slice

1. Add a structured `RouteLocation` store to runtime context.
2. Add web synchronization between that store and the browser location/history.
3. Add `Router` and `Route` with:
   - template-order first-match-wins
   - `path`
   - `default=true`
   - implicit `route` binding
   - `:param`
   - terminal `*`
4. Expose params as strings only in the first pass.
5. Expose both scoped and global locations on the match object.
6. Leave query-pattern matching, typed params, optional params, named catch-all bindings, and any router-specific link sugar for follow-up work.

## Deferred Follow-up Work

- optional single-segment params are intentionally out of phase 1; explicit sibling routes are enough to prove the core model first
- named catch-all bindings are intentionally out of phase 1; `route.remainder` is the only tail-reading surface for now
- query-aware pattern matching, typed params, and router-aware link ergonomics remain follow-up work after the base router ships

## Proposal Summary

The recommended direction is:

- structured route state internally
- browser URL as a web serialization of that state
- `Router` and `Route` as first-class Pax control-flow primitives
- implicit `route` match-object binding
- `:param` path captures plus terminal `*` for nested prefix/catch-all matching
- nested routers match against the nearest ancestor remainder by default
- nested consumers can still read the full global route through `route.global_location`
- `Link` stays router-agnostic while the web chassis handles History API synchronization
- route mutation remains imperative and outside template routing nodes
