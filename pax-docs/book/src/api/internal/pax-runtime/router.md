# router
<!-- summary: Runtime routing primitives used by declarative `Router` / `Route` control flow. -->
<!-- tags: api, pax-runtime -->

Runtime routing primitives used by declarative `Router` / `Route` control flow.

## Structs
### `CompiledRouteBranch`
Compiled route branch used by the runtime matcher.

#### Properties
##### `default`
Type: `bool`

Whether this branch is the fallback `default=true` branch.

##### `modal`
Type: `bool`

Whether this branch should stack over the previously active branch.

---

### `RouteLocation`
Structured application location shared across platforms.

On web targets this is serialized to and from `window.location`, while on
non-web targets it remains a platform-agnostic route state model.

#### Properties
##### `path_segments`
Type: `Vec`<`String`>

Decoded path segments, with `/` represented by an empty vector.

##### `query`
Type: `HashMap`<`String`, `Vec`<`String`>>

Query-string values keyed by parameter name, preserving repeated keys.

##### `fragment`
Type: `Option`<`String`>

Optional fragment without the leading `#`.

#### Implementations
##### `root`
<pre><code class="api-signature language-rust ignore">pub fn root() -&gt; Self</code></pre>

Returns the canonical root location.

##### `with_path_segments`
<pre><code class="api-signature language-rust ignore">pub fn with_path_segments(&amp;self, path_segments: Vec&lt;String&gt;) -&gt; Self</code></pre>

Clones this location while replacing only the path segments.

---

### `RouteMatch`
Structured match data exposed to the active route subtree as `route`.

#### Properties
##### `location`
Type: [`RouteLocation`](/api/internal/pax-runtime/router.md#routelocation)

Location as seen by the current router scope.

##### `global_location`
Type: [`RouteLocation`](/api/internal/pax-runtime/router.md#routelocation)

Full global application location.

##### `params`
Type: `HashMap`<`String`, `String`>

Named values captured from `:param` segments.

##### `consumed_segments`
Type: `usize`

Number of path segments consumed by the selected branch.

##### `remainder`
Type: `Vec`<`String`>

Remaining path tail left after the selected branch.

##### `is_exact`
Type: `bool`

Whether the selected branch consumed the entire scoped path.

#### Implementations
##### `remainder_location`
<pre><code class="api-signature language-rust ignore">pub fn remainder_location(&amp;self) -&gt; <a href="/api/internal/pax-runtime/router.md#routelocation">RouteLocation</a></code></pre>

Converts the remainder into a `RouteLocation` for nested router input.

---

### `RouterInstance`
#### Implementations
##### `instantiate_with_branches`
<pre><code class="api-signature language-rust ignore">pub fn instantiate_with_branches(args: <a href="/api/internal/pax-runtime/rendering.md#instantiationargs">InstantiationArgs</a>, branches: Vec&lt;<a href="/api/internal/pax-runtime/router.md#compiledroutebranch">CompiledRouteBranch</a>&gt;, branch_child_ranges: Vec&lt;Range&lt;usize&gt;&gt;) -&gt; Rc&lt;Self&gt;</code></pre>

Creates a router instance with precompiled route branches.

---

### `RouterProperties`
Internal router inputs carried into a `RouterInstance`.

#### Properties
##### `input_location`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<[`RouteLocation`](/api/internal/pax-runtime/router.md#routelocation)>

Location scoped to the current router.

##### `global_location`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<[`RouteLocation`](/api/internal/pax-runtime/router.md#routelocation)>

Full application location retained for diagnostics and coordination.

## Constants
### `INTERNAL_ROUTE_LOCATION_SYMBOL`
Internal stack symbol carrying the router input location for nested scopes.

---

### `INTERNAL_ROUTE_MATCH_SYMBOL`
Internal stack symbol carrying the current route match object.
