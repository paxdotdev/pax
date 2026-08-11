# Routing
<!-- summary: Declarative route matching, nested router scope, and URL-driven navigation. -->
<!-- tags: router, navigation, control-flow, templates -->

Pax routing is split cleanly into two jobs:

- `Router` and `Route` declaratively read the current location and choose UI.
- `Link` and `NodeContext::navigate_to(...)` imperatively write a new location.

That separation keeps route selection inside ordinary template control flow, while still letting Rust or UI events trigger navigation.

## Basic Shape

Use `Router` as a non-rendering control-flow node, and add `Route` branches under it.

```pax
<Router>
    <Route path="/">
        <Home />
    </Route>

    <Route path="/teams/:team_id/*">
        <TeamShell />
    </Route>

    <Route default=true>
        <NotFound />
    </Route>
</Router>
```

Currently `Router` supports:

- literal segments: `/settings/profile`
- single-segment params: `/teams/:team_id`
- terminal catch-all: `/settings/*`
- fallback branch: `default=true`

## The `route` Binding

The active route subtree receives an implicit `route` binding. The important fields are:

- `route.location`: the location scoped to the current router
- `route.global_location`: the full browser/native location
- `route.params`: values captured from `:param` segments
- `route.remainder`: the unmatched tail after the selected branch
- `route.is_exact`: whether the scoped path was fully consumed
- `route.consumed_segments`: how many scoped segments the branch consumed

The key opinion is that `route.location` is scoped by default. Nested routers usually care about their local remainder, not the original full path.

## Nested Routers

Nested routers match against the nearest ancestor remainder automatically.

```pax
<Router>
  <Route path="/teams/:team_id/*">
      <TeamHeader title={route.params.team_id} />
  
      <Router>
          <Route default=true>
              <Overview />
          </Route>
  
          <Route path="members/:member_id">
              <MemberDetail member_id={route.params.member_id} />
          </Route>
  
          <Route path="settings/*">
              <SettingsShell />
          </Route>
      </Router>
  </Route>
</Router>
```

Inside the nested router above:

- `members/:member_id` matches against the remainder after `/teams/:team_id/*`
- `route.location` is now local to the team workspace
- `route.global_location` still exposes the full app location

That makes nested route trees feel like ordinary component composition instead of global path parsing.

## Route Branch Kinds

`Router` can consume route branch components that declare the route-branch contract.
The standard branches are:

- `Route`: selects one mutually exclusive branch.
- `RouteCard`: stacks over the previous active branch and wraps its contents in a card-style lifecycle shell, so the previous route stays visible under a fading scrim while the card enters or exits.
- `RouteModal`: stacks the selected branch over the previous active branch. The previous branch stays mounted underneath a fading scrim while the modal branch is active, and the modal branch owns its own `@in` / `@out` transition.

Use `RouteModal` for pop-up or sheet routes where dismissing should reveal the previous route instead of remounting it. Set `duration` to match the modal content's movement when the branch declares custom `@in` / `@out` timelines.

## Writing Routes

Pax keeps route writes out of template matching nodes.

- Use `Link` for declarative navigation from templates.
- Use `NodeContext::navigate_to(...)` from Rust when navigation is event-driven or computed.

On web targets, same-origin navigation in the current tab can be synchronized through the browser History API, so nested route changes do not need to trigger full reloads.
Back and forward navigation are routed back into Pax through browser `popstate` and `hashchange` events.

## Web Route Metadata

Web builds can derive crawler-visible metadata from the same declarative route
tree that renders the application. Annotate a route in place with literal
`RouteMetadata`:

```pax
<Router>
    <Route
        path="/"
        metadata=RouteMetadata {
            title: "Wayfinder",
            description: "Plan a clear route across every screen.",
            index: true,
        }
    >
        <Home />
    </Route>

    <Route
        path="/notes/*"
        metadata=RouteMetadata {
            title: "Wayfinder Notes",
            description: "Product notes from the Wayfinder team.",
            index: true,
            social_image: "assets/notes-card.png",
            social_image_alt: "Wayfinder route map",
        }
    >
        <Notes />
    </Route>

    <Route
        default=true
        metadata=RouteMetadata {
            title: "Page not found — Wayfinder",
            description: "This page could not be found.",
            index: false,
        }
    >
        <NotFound />
    </Route>
</Router>
```

`title`, `description`, and `index` are required literals. `social_image` and
`social_image_alt` are optional, but must be supplied together. Metadata is
inherited by nested routes until a descendant declares its own block.

The compiler reads this topology for every target, but only a web build emits
artifacts. It writes:

- `route-metadata.json`, used to keep the document head synchronized after
  History API navigation
- a route-specific `index.html` for every concrete literal path
- the root `index.html`, which also acts as the application fallback

Parameterized routes remain symbolic because the compiler cannot enumerate
their values. A terminal catch-all such as `/notes/*` can still emit the
concrete prefix `/notes`. An indexable route cannot sit behind a dynamic
`if`, `repeat`, or slot boundary because its topology would not be
deterministic.

Configure site-wide values under `[package.metadata.pax.web]`:

```toml
[package.metadata.pax.web]
title = "Wayfinder"
site_name = "Wayfinder"
site_url = "https://example.com"
social_image = "assets/site-card.png"
social_image_alt = "Wayfinder"
```

`site_url` is required when a release web build contains an indexable concrete
route. It supplies canonical and absolute social URLs. Route-specific social
metadata overrides the site-wide pair.

### Hosting the Generated Routes

Serve a generated file when it exists. Rewrite every other application path
to `/index.html` while preserving the requested browser URL and returning the
application document. Do not redirect unknown paths to `/`, and do not use a
separate semantic `404.html`: the declarative default route decides whether a
preserved URL is valid and renders the application's not-found experience.

This is still client-side routing. A hard load initializes a fresh Pax
application, while same-origin `Link` and `navigate_to(...)` transitions keep
the current session and update the document head in place.

## Example

<pax-example
  path="router-playground"
  title="Router Playground"
  height="760"
  files="src/lib.pax,src/lib.rs,src/guide_panel.pax,src/team_panel.pax,src/route_inspector.rs,src/route_inspector.pax">
</pax-example>
