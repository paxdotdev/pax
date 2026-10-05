# api
<!-- summary: API docs for pax-runtime::api. -->
<!-- tags: api, pax-runtime -->

## Structs
### `NodeContext`
This runtime owner stays on the UI thread. Transfer properties or delivery
handles to workers instead.

```compile_fail
fn requires_send<T: Send>() {}
requires_send::<pax_runtime::api::NodeContext>();
```
Runtime context passed into user component lifecycle methods and event handlers.

For an inline binding, Rust `self` belongs to the component that authored
the binding, while this context describes the child receiving the call.
Bounds, expression stack access, subscriptions, and async lifetime remain target-local.
Component-level bindings receive their own component's state and context.

Child-related fields intentionally separate semantic payload from engine
transport:

- `projected_children` is the raw transport family used by `Slot`
- `received_children` is the normalized semantic payload that this node
  should treat as content from its caller
- `retained_received_children` are former received children kept alive only
  so `@out` transitions can finish

A node's own private template or primitive-assembled structure is
intentionally not surfaced here as a first-class "child family" for
container consumers.

#### Properties
##### `expanded_node`
Type: `Weak`<`ExpandedNode`>

##### `slot_index`
Type: `LocalProperty`<`Option`<`usize`>>

slot index of this node in its container

##### `expression_stack`
Type: `Rc`<[`RuntimePropertiesStackFrame`](../../../api/internal/pax-runtime/properties.md#runtimepropertiesstackframe)>

##### `containing_component`
Type: `Weak`<`ExpandedNode`>

Reference to the ExpandedNode of the component containing this node

##### `elapsed_frames`
Type: `LocalProperty`<`u64`>

The current global engine frame count.

##### `elapsed_millis`
Type: `LocalProperty`<`u64`>

The current global engine wall-clock time in milliseconds.

##### `gyro`
Type: `LocalProperty`<[`Gyro`](../../../api/pax-runtime-api/platform.md#gyro)>

Current device orientation sensor reading.

##### `accel`
Type: `LocalProperty`<[`Accel`](../../../api/pax-runtime-api/platform.md#accel)>

Current device accelerometer reading.

##### `bounds_parent`
Type: `LocalProperty`<(`f64`, `f64`)>

The bounds of this element's immediate container (parent) in px

##### `bounds_self`
Type: `LocalProperty`<(`f64`, `f64`)>

The bounds of this element in px

##### `measured_size`
Type: `LocalProperty`<`Option`<(`f64`, `f64`)>>

Measured bounds resolved by the chassis or container layout for this node.

##### `subtree_layout_hull`
Type: `LocalProperty`<[`LayoutHull`](../../../api/internal/pax-runtime/layout.md#layouthull)>

Node-local subtree layout hull published by the engine for container measurement.

##### `platform`
Type: [`Platform`](../../../api/pax-runtime-api/platform.md#platform)

Current platform (Web/Native) this app is running on

##### `os`
Type: [`OS`](../../../api/pax-runtime-api/platform.md#os)

Current os (Android/Windows/Mac/Linux) this app is running on

##### `target`
Type: `LocalProperty`<[`TargetInfo`](../../../api/pax-runtime-api/platform.md#targetinfo)>

Derived target facts for platform/OS checks.

##### `viewport`
Type: `LocalProperty`<[`Viewport`](../../../api/pax-runtime-api/platform.md#viewport)>

Derived viewport facts for size and orientation checks.

##### `projected_children_count`
Type: `LocalProperty`<`usize`>

The number of projected children available to this node.

This is the raw transport count used by slot-driven implementations.
Container-style consumers usually want `received_children_count`
instead.

##### `node_transform_and_bounds`
Type: [`TransformAndBounds`](../../../api/internal/pax-runtime/layout.md#transformandbounds)<[`NodeLocal`](../../../api/internal/pax-runtime/engine/node_interface.md#nodelocal), [`Window`](../../../api/pax-runtime-api/platform.md#window)>

The transform of this node in the global coordinate space

##### `projected_children`
Type: `LocalProperty`<`Vec`<`Rc`<`ExpandedNode`>>>

Children projected into this node from the containing component.

Projection is an engine transport mechanism. Consumers that want the
semantic payload owned by this node should prefer `received_children`.

##### `projected_children_changed`
Type: `LocalProperty`<()>

A structural invalidation signal for projected children.

##### `received_children`
Type: `LocalProperty`<`Vec`<`Rc`<`ExpandedNode`>>>

Semantic payload children received by this node from its caller.

This is the canonical "content" view for container-style logic. It
excludes private encapsulated implementation children and also excludes
exit-retained payload nodes, which instead appear in
`retained_received_children`.

##### `received_children_count`
Type: `LocalProperty`<`usize`>

Convenience count derived from `received_children`.

##### `received_children_changed`
Type: `LocalProperty`<()>

A structural invalidation signal for `received_children`.

Prefer this or `received_children` itself for structural subscriptions
that must react to reorders as well as insertions and removals.

##### `retained_received_children`
Type: `LocalProperty`<`Vec`<`Rc`<`ExpandedNode`>>>

Received children retained only so exit transitions can finish.

These are no longer part of the active semantic payload, but some
containers still need to place them as ghosts or overlays while their
`@out` transitions run.

##### `retained_received_children_changed`
Type: `LocalProperty`<()>

A structural invalidation signal for `retained_received_children`.

#### Implementations
##### `application`
<pre><code class="api-signature language-rust ignore">pub fn application(&amp;self) -&gt; <a href="../../../api/pax-runtime-api/application.md#appcontext">AppContext</a></code></pre>

Access services belonging to this application instance.

##### `async_scope`
<pre><code class="api-signature language-rust ignore">pub fn async_scope(&amp;self) -&gt; Result&lt;<a href="../../../api/pax-runtime-api/async_runtime.md#asyncscope">AsyncScope</a>, <a href="../../../api/pax-runtime-api/async_runtime.md#asyncerror">AsyncError</a>&gt;</code></pre>

Lifetime of this exact context node, ending at actual unmount. Retain a
component mount scope when later handlers receive child-control contexts.
A saved context cannot acquire the scope of a later mount of that node.

##### `clear_subscriptions`
<pre><code class="api-signature language-rust ignore">pub fn clear_subscriptions(&amp;self)</code></pre>

Remove all subscriptions registered on this node.

##### `dispatch_event`
<pre><code class="api-signature language-rust ignore">pub fn dispatch_event(&amp;self, identifier: &amp;&#39;static str) -&gt; Result&lt;(), String&gt;</code></pre>

Queue a named custom event from this component for dispatch at the end of the tick.

##### `elapsed_time_millis`
<pre><code class="api-signature language-rust ignore">pub fn elapsed_time_millis(&amp;self) -&gt; u128</code></pre>

Milliseconds elapsed according to the chassis-provided clock.

##### `get_node_interface`
<pre><code class="api-signature language-rust ignore">pub fn get_node_interface(&amp;self) -&gt; Option&lt;<a href="../../../api/internal/pax-runtime/engine/node_interface.md#nodeinterface">NodeInterface</a>&gt;</code></pre>

Return the interface for this node's containing component, when present.

##### `get_screenshot_map`
<pre><code class="api-signature language-rust ignore">pub fn get_screenshot_map(&amp;self) -&gt; Rc&lt;RefCell&lt;HashMap&lt;u32, <a href="../../../api/internal/pax-message/index.md#screenshotdata">ScreenshotData</a>&gt;&gt;&gt;</code></pre>

Shared map where completed screenshot captures are published by id.

##### `local_point`
<pre><code class="api-signature language-rust ignore">pub fn local_point(&amp;self, p: Point2&lt;<a href="../../../api/pax-runtime-api/platform.md#window">Window</a>&gt;) -&gt; Point2&lt;<a href="../../../api/internal/pax-runtime/engine/node_interface.md#nodelocal">NodeLocal</a>&gt;</code></pre>

Convert a window-space point into this node's local coordinate space, including any
presentation offsets inherited from ancestor scrollers.

##### `navigate_to`
<pre><code class="api-signature language-rust ignore">pub fn navigate_to(&amp;self, url: &amp;str, target: <a href="../../../api/pax-runtime-api/drawing.md#navigationtarget">NavigationTarget</a>)</code></pre>

Ask the chassis to navigate to a URL.

On web targets, same-origin navigation in the current tab can be handled
through the browser History API and routed back into Pax without a full
page reload. Cargo metadata's
`[package.metadata.pax.web].server_owned_prefixes` delegates matching paths
to ordinary browser navigation before changing application history.
Other targets use the active chassis navigation behavior.

##### `screenshot`
<pre><code class="api-signature language-rust ignore">pub fn screenshot(&amp;self, id: u32)</code></pre>

Request a screenshot capture from the chassis, keyed by caller-provided id.

##### `set_cursor`
<pre><code class="api-signature language-rust ignore">pub fn set_cursor(&amp;self, cursor: <a href="../../../api/pax-runtime-api/cursor.md#cursorstyle">CursorStyle</a>)</code></pre>

Ask the chassis to display the requested cursor over the app surface.

##### `subscribe`
<pre><code class="api-signature language-rust ignore">pub fn subscribe(&amp;self, dependencies: &amp;[UntypedProperty], f: impl Fn() + &#39;static)</code></pre>

Attach a dependency subscription whose callback runs when any dependency dirties.

##### `provide_store`
<pre><code class="api-signature language-rust ignore">pub fn provide_store&lt;T: <a href="../../../api/pax-runtime-api/store.md#store">Store</a>&gt;(&amp;self, store: T) -&gt; Result&lt;(), <a href="../../../api/pax-runtime-api/store.md#storeerror">StoreError</a>&gt;</code></pre>

Publish a store on this context's mounted node, keyed by its concrete Rust type.

Register in the component's own mount handler to give each component
instance an independent provider. In an inline descendant handler, the
context identifies that descendant even when Rust `self` is its authoring
component. The context's node, not `self`, owns the registration.

The owner and its logical descendants can find the value with `with_store`.
This includes private template children and caller-supplied children;
private wrappers around a slot do not change projected content's ancestry.
Siblings and ancestors cannot discover this registration.

Providing the same type again on the same owner replaces its entry for
future lookups. Previously cloned handles keep their original state; the
registry is not reactive. Prefer updating properties in a stable store.
Providers are cleared at final unmount, after unmount handlers run.

Returns `StoreError::ExpiredScope` for an unmounted or stale context, or
`StoreError::BorrowConflict` if the entry being replaced is borrowed.

##### `with_store`
<pre><code class="api-signature language-rust ignore">pub fn with_store&lt;T: <a href="../../../api/pax-runtime-api/store.md#store">Store</a>, V&gt;(&amp;self, f: impl FnOnce(&amp;mut T) -&gt; V) -&gt; Result&lt;V, <a href="../../../api/pax-runtime-api/store.md#storeerror">StoreError</a>&gt;</code></pre>

Borrow the nearest provider of `T`, starting with this context's node.

Lookup follows logical template ancestry, independently of expression
scope and render parents. A nearer same-type provider shadows an outer
one; unrelated types remain visible. It never searches siblings or
descendants. Two consumers share state when they resolve the same provider.

The closure exclusively borrows that store for its duration. Clone a
needed `Property` handle inside the closure, then read or update it after
the borrow ends. This avoids conflicts if subsequent work looks up the
same store. The returned value can be a handle or an owned snapshot, but
cannot borrow from the store.

Returns `StoreError::Missing` when no provider exists,
`StoreError::ExpiredScope` when the context or an ancestor's mount has
ended, and `StoreError::BorrowConflict` when the nearest provider is
already borrowed. A conflict or expired scope never falls back to an
outer provider. Saved contexts cannot discover a later mount's stores.
