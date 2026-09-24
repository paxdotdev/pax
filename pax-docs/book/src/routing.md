# Routing
<!-- summary: Organize screens around routes, navigate from templates or Rust, compose nested paths, and publish web route metadata. -->
<!-- tags: router, navigation, history, routes, metadata, control-flow -->

A useful URL names a place in your application: a project, a member's profile,
or the settings panel someone wants to share. Pax connects that location to a
component tree. On the web, the location follows the browser URL and its
history. On macOS, iOS, and iPadOS, the same route tree can select screens
using an in-app location.

This chapter builds on [Templates](template-language.md),
[Events and Rust](event-handling-rust.md), and
[Components and Composition](components-composition.md). Start with ordinary
routes, then add nested screens or route transitions as the application grows.

## Example

The Router Playground combines a persistent navigation area, nested guide
and team routes, and an inspector for the active route data. Open
`src/route_outlet.pax` to find the outer router; `src/team_panel.pax` shows
the nested router and its navigation links.

The example starts at Landing. Open **Menu** in the narrow layout to find
the guide, team screens, and fallback routes.

<pax-example
  path="router-playground"
  title="Router Playground"
  height="760"
  files="src/route_outlet.pax,src/team_panel.pax,src/sidebar_nav.rs,src/route_inspector.rs,src/route_inspector.pax,src/lib.pax,src/lib.rs">
</pax-example>

Choose **Open standalone** to explore the playground with its own browser
address bar:

1. Open a team member and compare the outer and inner scope shown by the
   inspectors.
2. Follow a team settings link. The outer team remains selected while the
   inner route and remainder change.
3. Visit an unknown path and inspect the fallback's unconsumed segments.
4. Use Back and Forward, then reload the selected screen.

The docs host keeps the example's route in its `pax_route` URL query parameter,
so navigation and reload stay inside the example's directory. Its navigation
does not change the address of this documentation page. A regular standalone
Pax application uses browser paths, as described below. The playground demonstrates runtime
routing. Its responsive route trees are not a template for indexable static
metadata; see [Web route metadata](#web-route-metadata) for that purpose.

## Routes and history

Three pieces work together:

- `Router` reads the current location and chooses a branch.
- `Route` declares a path and the content that belongs there.
- `Link` or Rust's `NodeContext::navigate_to` requests a new location.

Navigation updates the location; the router reacts by selecting the matching
subtree. Your event handler does not need to hide the old page and show the
new one manually.

On the web, same-origin navigation in the current tab updates browser history
without restarting Pax, except for [server-owned paths](#server-owned-web-paths).
Back and Forward feed application-location changes into the same router.
A fresh visit or browser reload asks the host for the requested document, so
[direct-link hosting](#hosting-the-generated-routes) is part of making a routed
app usable.

## Basic shape

`Router` has no visual surface of its own. Place it inside the layout area
that should contain the selected screen.

Here is a small two-page application. In `src/lib.rs`:

```rust
use pax_kit::*;

#[pax]
#[main]
#[file("lib.pax")]
pub struct App {}
```

In `src/lib.pax`:

```pax
<Group x=24px y=24px width={100% - 48px} height={100% - 48px}>
    <Link width=100px height=36px url="/">
        <Text width=100% height=100% text="Home" />
    </Link>
    <Link x=120px width=100px height=36px url="/notes">
        <Text width=100% height=100% text="Notes" />
    </Link>

    <Group y=64px width=100% height={100% - 64px}>
        <Router>
            <Route path="/">
                <Text width=100% height=48px text="A place for your ideas." />
            </Route>
            <Route path="/notes">
                <Text width=100% height=48px text="Your field notes." />
            </Route>
            <Route default=true>
                <Text width=100% height=48px text="This page could not be found." />
            </Route>
        </Router>
    </Group>
</Group>
```

Run it with `pax-cli run --target web`. Follow the links, then use the browser's
Back button. As the pages become more substantial, replace the text in each
branch with a component such as `<Home />` or `<Notes />`. The navigation can
stay outside the router and remain mounted as the page changes.

### Matching paths

Route patterns are literal strings. The supported forms are:

| Pattern | Matches |
| --- | --- |
| `/` | The root of the current router's scope. |
| `/notes` | That exact path. |
| `/teams/:team_id` | One segment captured as `team_id`. |
| `/teams/:team_id/*` | The team path and any remaining segments. |
| `default=true` | A fallback when no explicit path matches. |

A `*` must occupy the final segment. It can match an empty tail, so
`/teams/:team_id/*` also matches `/teams/design`. Without the `*`, a pattern
must consume the complete scoped path.

**The first matching explicit branch wins.** Put a specific route such as
`/teams/new` before `/teams/:team_id`, and put a broad catch-all after more
specific routes. There is no automatic specificity ranking. A default branch
is considered only after all explicit branches fail, regardless of where the
default appears in the file. Prefer one default per router. With no match and
no default, that router contributes no active content.

Leading and trailing slashes do not change how a route pattern matches its
scope. In a nested router, `members/:member_id` and `/members/:member_id`
both describe a local path; the leading slash does not escape to the root.

## Writing routes

### Links in templates

Wrap the visible content of a navigation control in a `Link`:

```pax
<Link width=200px height=44px url="/teams/design">
    <Text width=100% height=100% text="Open the design team" />
</Link>
```

The default target is `Target::Current`. Use `target=Target::New` when an
external web destination should open in a new browsing context:

```pax
<Link width=200px height=44px url="https://pax.dev" target=Target::New>
    <Text width=100% height=100% text="Visit Pax" />
</Link>
```

Give the wrapper and its content a useful interaction area. Link participates
in Pax's event system; see [Accessibility and Native Controls](accessibility-native-controls.md)
for the current accessibility boundaries.

### Navigation from Rust

Use `navigate_to` after an action, or when Rust determines the destination.
For example, bind a button to this handler on your component:

```rust
impl App {
    pub fn open_team(&mut self, ctx: &NodeContext, _event: Event<ButtonClick>) {
        ctx.navigate_to(
            "/teams/design?view=board#activity",
            NavigationTarget::Current,
        );
    }
}
```

```pax
<Button width=180px height=36px label="Open team"
    @button_click=self.open_team />
```

This requests navigation through the active platform interface. Use
root-relative paths such as `/teams/design` for destinations within the app.
On the web, a relative destination is resolved against the current browser
URL, **not** the nearest router's scope. Navigating to `settings` from
`/teams/design` can therefore differ from navigating there from
`/teams/design/`.

For a web `Current` destination, matching scheme, host, and port keep the
navigation in the Pax session unless its path is configured as server-owned.
Other origins and `New` destinations use ordinary browser navigation. The URL should describe the intended location;
route matching does not validate that a user may access its data. Perform
authorization in the systems that own that data.

### Server-owned web paths

A site can combine a Pax application with standalone documents such as a
static blog. Declare the paths that the HTTP server or CDN should resolve in
the application's `Cargo.toml`:

```toml
[package.metadata.pax.web]
server_owned_prefixes = ["/blog", "/downloads"]
```

Existing links and Rust handlers then use their ordinary URLs:

```pax
<Link url="/blog/pax-0-39-0/" target=Target::Current>
    <Text text="Read the announcement" />
</Link>
```

The same policy applies to `ctx.navigate_to("/blog/", NavigationTarget::Current)`.
On the web, Pax initiates ordinary current-tab browser navigation before
changing application history or notifying Router. The destination may be a
document, redirect, download, or another application. Queries and fragments are
preserved. Leaving for a document unloads the current app; Back may restore it
from the browser cache or start it again. In a query-backed embed, this navigates
the current frame to the real URL instead of changing its `pax_route` parameter.

Prefixes match complete, case-sensitive path segments. `/blog` covers `/blog`,
`/blog/`, and descendants, but not `/blogger` or `/Blog`. A trailing slash is
normalized away; `/` delegates every path. Entries must be root-relative paths,
without wildcards, query strings, fragments, backslashes, whitespace, empty
segments, or `.`/`..` segments. Unicode paths and valid percent escapes are
accepted. Unreserved ASCII escapes compare equivalently (`/%62log` is `/blog`);
encoded separators stay encoded (`/blog%2Fpost` is not beneath `/blog`). This is
a navigation policy, not an authorization boundary.

The default is an empty list. The setting applies only to web builds, in both
debug and release; native routing and new-tab links retain their usual behavior.
Rebuild and restart after changing Cargo metadata. The compiler embeds the
policy as JSON in every generated application entry and writes
`pax-web-config.json` for local serving. Custom entry documents must retain the
generated `pax-web-config` script element for client navigation to use it.

The setting does not configure a production server or reverse proxy. Serve the
declared paths separately, using [web public files](targets-build-deploy.md#web-public-files)
or a CDN origin. `pax-cli run` serves existing generated/public files there and
returns 404 for missing files instead of the application history fallback.
Remove conflicting Router metadata entries: a public document cannot overwrite
a generated application entry. The HTTP host handles direct links without
initializing Pax first.

### Native locations

On macOS, iOS, and iPadOS, current-target relative paths update Pax's virtual
in-app location. Query parameters and fragments travel with it. External
destinations are handed to the operating system to open.

A native router does not provide browser chrome, a native back-stack control,
or automatic universal-link registration. Supply the application's navigation
controls and configure any OS-level deep-link integration separately. Prefer
explicit destinations for a portable “Back to team” control.

## The `route` binding

Inside the selected branch, Pax supplies a reactive `route` value. Its fields
describe this match:

| Field | Meaning |
| --- | --- |
| `route.location` | The structured location seen by this router. |
| `route.global_location` | The full application location. |
| `route.params` | Named captures from this branch's `:param` segments. |
| `route.remainder` | A list of path segments left for a nested router. |
| `route.consumed_segments` | The number of segments consumed before that remainder. |
| `route.is_exact` | Whether the remainder is empty. |

For example, inside `/teams/:team_id/*`:

```pax
<Text width=100% height=36px text={"Team: " + route.params.team_id} />
```

Both location objects contain `path_segments`, `query`, and `fragment`.
For the web URL
`/teams/design/members/ada?tag=ui&tag=motion#activity`:

- The global path segments are `["teams", "design", "members", "ada"]`.
- `query` maps `tag` to `["ui", "motion"]`; repeated keys retain their values.
- `fragment` is an optional string containing `"activity"`, without `#`.

The web interface decodes URL components before delivering these values.
Query and fragment data do not decide which path pattern matches. They can
drive the selected screen's filters or other UI. Check optional values and
missing keys before consuming them, and encode user-supplied values when
constructing URLs.

A fragment is location data. Scrolling a Pax `Scroller` to a corresponding
piece of content is application behavior; it is not implied by adding
`#activity` to a route. See [Scrolling and Viewports](scrolling-viewports.md).

## Nested routers

A nested router receives the nearest ancestor route's remainder. This lets a
component own the routes beneath its part of the application.

For example, let the outer route supply the team identifier to `TeamPanel`:

```pax
<Router>
    <Route path="/teams/:team_id/*">
        <TeamPanel team_id={route.params.team_id} />
    </Route>
    <Route default=true>
        <NotFound />
    </Route>
</Router>
```

Declare `pub team_id: Property<String>` on `TeamPanel`. Its template can use
that property for persistent team chrome, while its own router chooses the
team's inner screen:

```pax
<Group width=100% height=100%>
    <Text width=100% height=40px text={"Team: " + self.team_id} />
    <Group y=56px width=100% height={100% - 56px}>
        <Router>
            <Route path="/">
                <TeamOverview />
            </Route>
            <Route path="members/:member_id">
                <MemberDetail member_id={route.params.member_id} />
            </Route>
            <Route path="settings/*">
                <TeamSettings />
            </Route>
            <Route default=true>
                <TeamNotFound />
            </Route>
        </Router>
    </Group>
</Group>
```

The screen components here stand for your application's content. For
`/teams/design/members/ada`, scope changes in two steps:

| Router | Input segments | Captures | Remainder |
| --- | --- | --- | --- |
| Outer | `teams / design / members / ada` | `team_id = design` | `members / ada` |
| Team panel | `members / ada` | `member_id = ada` | Empty |

The inner `route` describes the inner match. Its params do not automatically
merge the outer route's captures. Passing `team_id` into the component gives
it a stable, explicit name alongside the inner `member_id`.
`route.global_location` continues to describe the complete location at both
levels. Query and fragment data are preserved when the path is scoped.

Use `path="/"` for a team's overview and `default=true` for its unknown
subpaths. Using a default alone for the overview would also show it for
unrecognized tails. The ancestor's terminal `*` is what makes additional
path segments available to the inner router.

## Route state and lifetime

Pax uses the route declaration and its captured parameters to identify an
active route instance.

- Changing only query data, a fragment, or a catch-all remainder updates the
  existing instance's reactive route context.
- Changing a captured parameter creates a new instance. Moving from
  `/teams/design` to `/teams/engineering` creates a new team subtree.
- Changing to a different ordinary route removes the old branch, subject to
  its exit lifecycle. Returning later generally creates it again.

This is why a team shell can keep local state while its inner router switches
from members to settings. State that must survive leaving the team should
have an owner outside that route: a longer-lived component, a local store
provided above the router, or your application's persistence layer.
[Components and Composition](components-composition.md#shared-state-farther-down-the-tree)
explains the store pattern.

Browser history records locations, not snapshots of every component's
properties. Do not rely on Back or a page reload to restore unsaved form state.

## Route branch kinds

Use an ordinary `Route` for mutually exclusive screens. `RouteCard` and
`RouteModal` add a retained background for route-driven overlays.

A card supplies its own sliding shell:

```pax
<Router>
    <Route path="/">
        <Home />
    </Route>
    <RouteCard path="/details" edge=RouteCardEdge::Trailing duration=300ms>
        <Details />
    </RouteCard>
</Router>
```

Navigate from Home to `/details` to keep Home mounted underneath the incoming
card. The card slides from the chosen edge while a scrim fades over the
background. Available edges are `Leading`, `Trailing`, `Top`, and `Bottom`.
`duration` controls the transition; `scrim_opacity` controls the dimming.

`RouteModal` retains the previous route and supplies the fading scrim, while
its contents own their layout and any `@in` / `@out` movement. Set its
`duration` to coordinate the scrim with those content transitions.
[Animation and Motion](animation-motion.md) covers lifecycle timelines.

Provide a close control that navigates to the intended background route.
Returning to that retained route can reveal its existing state without
remounting it. The scrim does not supply an automatic close action.

An overlay entered by a fresh URL has no previous route to retain. Design
that direct-load state deliberately: include the context the overlay needs,
or place essential shared chrome outside the router. A browser history entry
alone cannot reconstruct the previous mounted screen.

## Custom route branches

You can define route branch types in your application or a reusable Rust
crate. The `#[route_branch(...)]` attribute tells the compiler which
properties describe the path and fallback, and whether the selected branch
retains the previous one underneath. No `pax-std` enum change is required.

A custom branch is useful when several routes share presentation: a padded
panel, a branded sheet, or a particular enter/exit treatment. It remains an
ordinary template-backed component, with properties, slots, and lifecycle
timelines.

For example, create `src/panel_route.rs`:

```rust
use pax_kit::*;

#[pax]
#[file("panel_route.pax")]
#[route_branch(path = "pattern", default = "fallback")]
pub struct PanelRoute {
    pub pattern: Property<String>,
    pub fallback: Property<bool>,
}
```

Here `pattern` and `fallback` are names chosen by the component author. The
attribute maps them to the router's existing contract. They could also be
named `path` and `default`, as in the standard branches.

Give it a `src/panel_route.pax` template that presents its caller's content:

```pax
<Group x=20px y=20px width={100% - 40px} height={100% - 40px}>
    slot()
</Group>
<Rectangle width=100% height=100%
    fill=rgb(246, 241, 230) corner_radius=16 />
```

Expose the type from your application's `lib.rs`:

```rust
pub mod panel_route;
pub use panel_route::PanelRoute;
```

Then use it directly under a router, in the layout area for the screen:

```pax
<Router>
    <PanelRoute pattern="/notes/:slug">
        <Text width=100% height=40px text={"Note: " + route.params.slug} />
    </PanelRoute>
    <PanelRoute fallback=true>
        <Text width=100% height=40px text="No matching note." />
    </PanelRoute>
</Router>
```

Pax selects the branch and preserves its component shell. The slotted
content receives the route binding, while the shell supplies padding and a
background. Its other properties and timelines work as they do on ordinary
components. See [Slots](components-composition.md#slots) for caller scope
and projection.

The default policy replaces the previous active route. To make a custom
branch retain the previous route, add `modal = true` to its attribute:

```rust
#[route_branch(path = "pattern", default = "fallback", modal = true)]
```

This is a policy for that branch **type**. It does not automatically add a
scrim, a close button, or an animation; the component supplies those parts.
Use the standard [RouteModal and RouteCard implementations](https://github.com/paxproject/pax/blob/dev/pax-std/src/core/router.rs)
as examples of retained-route presentation.

Custom branches use the same literal patterns, declaration-order matching,
nested scope, and parameter-based identity as the standard branches. New
matching syntax or a different retention policy would need changes to the
router implementation. For deeper runtime behavior, Pax also exposes
[primitive authoring](primitives.md#authoring-primitives), a
separate extension point that custom route presentation usually does not need.

## Web route metadata

A public web route also needs a title and description before someone
interacts with the app. Add literal `RouteMetadata` to the route that owns
that page:

```pax
<Router>
    <Route path="/" metadata=RouteMetadata {
        title: "Field notes",
        description: "A small home for observations and ideas.",
        index: true,
    }>
        <Home />
    </Route>
    <Route path="/notes/*" metadata=RouteMetadata {
        title: "Notes — Field notes",
        description: "Observations from the field.",
        index: true,
        social_image: "assets/notes-card.png",
        social_image_alt: "An open field notebook",
    }>
        <Notes />
    </Route>
    <Route default=true metadata=RouteMetadata {
        title: "Page not found — Field notes",
        description: "This page could not be found.",
        index: false,
    }>
        <NotFound />
    </Route>
</Router>
```

Supply your own social image at the referenced asset path. `title` and
`description` are required, nonempty string literals; `index` is a required
boolean literal. The optional `social_image` and `social_image_alt` fields
must appear together. These are build-time declarations, so property
bindings and computed titles are not supported in this metadata block.

A nested route inherits the nearest metadata block until it declares its own.
A replacement block must still provide all required fields. Keep indexable
route topology static: placing it behind an `if`, `for`, or projected slot
prevents deterministic metadata generation and causes the web build to fail.
This includes passing an indexable router as projected children to a
template-backed layout component. Keep the router outside that projection;
responsive layout can remain inside a statically declared route.

### Site-wide values

Set the public site identity in the project's `Cargo.toml`:

```toml
[package.metadata.pax.web]
title = "Field notes"
site_name = "Field notes"
site_url = "https://notes.example.com"
social_image = "assets/site-card.png"
social_image_alt = "Field notes"
```

The route's social-image pair overrides the site-wide pair. `site_url` must
be an absolute HTTP(S) URL without a query or fragment; it supplies canonical
URLs and absolute social-image URLs. A release web build containing an
indexable concrete route requires it. Use the actual production URL for a
published build, and keep the referenced images in the published output.

### Generated entries and indexing

The web build emits `route-metadata.json` and route-specific entry HTML for
concrete paths that have effective metadata. For the preceding example:

```text
.pax/build/release/web/
    index.html
    notes/
        index.html
    route-metadata.json
    …the rest of the application bundle
```

The entry documents contain the title, description, robots directive,
canonical URL when configured, and Open Graph/Twitter metadata. After
in-app navigation, the web interface updates the document head from the
catalog without restarting the Pax session.

Concrete and symbolic paths have different limits:

- A literal `/notes` can receive its own entry document.
- A terminal catch-all `/notes/*` can emit the concrete prefix `/notes`.
  Arbitrary deeper URLs do not become separate pages.
- A parameterized `/notes/:slug` remains symbolic. The compiler does not
  enumerate records or generate an entry document for every possible slug.
- At runtime, symbolic matches and catch-all tails receive `noindex` and no
  canonical link. An indexable directive applies only at the route's concrete
  path. Declare concrete routes for pages that need individually generated
  metadata.

The HTML contains an application entry and its metadata; the screen itself
still renders when Pax starts. An unknown direct URL initially receives the
fallback document's head, then the running app applies the default route's
metadata. Account for this when checking previews from crawlers that do not
execute the app. This mechanism does not generate a sitemap or implement
server-side rendering.

### Hosting the generated routes

Serve a generated entry or real public file when it exists. For other
application paths, serve `/index.html` while preserving the requested browser
URL. A redirect to `/` would discard the location before the router reads it.

The default route owns the in-app not-found screen. The build does not create
a separate semantic `404.html`, and choosing a fallback route cannot change
the status of an HTTP response already delivered by the server. Keep
missing asset responses separate from application fallback.

The generator also sets the entry documents' HTML base URL so nested entries
can load the application bundle. Start with a domain-root deployment; a
subdirectory deployment requires the browser paths, site URL, and hosting
prefix to agree. See
[Targets, Build, and Deployment](targets-build-deploy.md#direct-links-and-the-base-url)
for the hosting configuration and its verification checklist.


## Read more

- [Components and Composition](components-composition.md) — page components,
  shared shells, and state ownership.
- [State and Properties](state-properties.md) — reactive state that follows
  navigation.
- [Animation and Motion](animation-motion.md) — enter and exit lifecycles.
- [Targets, Build, and Deployment](targets-build-deploy.md) — release output,
  direct links, and static hosting.
- [Router, Route, RouteCard, and RouteModal reference](api/pax-std/core/router.md)
  and [Link reference](api/pax-std/core/link.md) — individual component fields.
