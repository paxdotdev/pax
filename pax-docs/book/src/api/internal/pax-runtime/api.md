# api
<!-- summary: API docs for pax-runtime::api. -->
<!-- tags: api, pax-runtime -->

## Structs
### `NodeContext`
Runtime context passed into user component lifecycle methods and event handlers.

#### Properties
##### `expanded_node`
Type: `Weak`<`ExpandedNode`>

##### `slot_index`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<`Option`<`usize`>>

slot index of this node in its container

##### `local_stack_frame`
Type: `Rc`<[`RuntimePropertiesStackFrame`](/api/internal/pax-runtime/properties.md#runtimepropertiesstackframe)>

Stack frame of this component, used to look up stores

##### `containing_component`
Type: `Weak`<`ExpandedNode`>

Reference to the ExpandedNode of the component containing this node

##### `frames_elapsed`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<`u64`>

The current global engine tick count

##### `bounds_parent`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<(`f64`, `f64`)>

The bounds of this element's immediate container (parent) in px

##### `bounds_self`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<(`f64`, `f64`)>

The bounds of this element in px

##### `platform`
Type: [`Platform`](/api/pax-runtime-api/platform.md#platform)

Current platform (Web/Native) this app is running on

##### `os`
Type: [`OS`](/api/pax-runtime-api/platform.md#os)

Current os (Android/Windows/Mac/Linux) this app is running on

##### `slot_children_count`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<`usize`>

The number of slot children provided to this component template

##### `node_transform_and_bounds`
Type: [`TransformAndBounds`](/api/internal/pax-runtime/layout.md#transformandbounds)<[`NodeLocal`](/api/internal/pax-runtime/engine/node_interface.md#nodelocal), [`Window`](/api/pax-runtime-api/platform.md#window)>

The transform of this node in the global coordinate space

##### `slot_children`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<`Vec`<`Rc`<`ExpandedNode`>>>

Slot children of this node

##### `slot_children_attached_listener`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<()>

A property that can be depended on to dirty when a slot child is attached

#### Implementations
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
<pre><code class="api-signature language-rust ignore">pub fn get_node_interface(&amp;self) -&gt; Option&lt;<a href="/api/internal/pax-runtime/engine/node_interface.md#nodeinterface">NodeInterface</a>&gt;</code></pre>

Return the interface for this node's containing component, when present.

##### `get_screenshot_map`
<pre><code class="api-signature language-rust ignore">pub fn get_screenshot_map(&amp;self) -&gt; Rc&lt;RefCell&lt;HashMap&lt;u32, <a href="/api/internal/pax-message/index.md#screenshotdata">ScreenshotData</a>&gt;&gt;&gt;</code></pre>

Shared map where completed screenshot captures are published by id.

##### `local_point`
<pre><code class="api-signature language-rust ignore">pub fn local_point(&amp;self, p: Point2&lt;<a href="/api/pax-runtime-api/platform.md#window">Window</a>&gt;) -&gt; Point2&lt;<a href="/api/internal/pax-runtime/engine/node_interface.md#nodelocal">NodeLocal</a>&gt;</code></pre>

Convert a window-space point into this node's local coordinate space.

##### `navigate_to`
<pre><code class="api-signature language-rust ignore">pub fn navigate_to(&amp;self, url: &amp;str, target: <a href="/api/pax-runtime-api/drawing.md#navigationtarget">NavigationTarget</a>)</code></pre>

Ask the chassis to navigate to a URL.

##### `peek_local_store`
<pre><code class="api-signature language-rust ignore">pub fn peek_local_store&lt;T: <a href="/api/pax-runtime-api/store.md#store">Store</a>, V&gt;(&amp;self, f: impl FnOnce(&amp;mut T) -&gt; V) -&gt; Result&lt;V, String&gt;</code></pre>

Borrow the nearest stack-local store of type `T`.

##### `push_local_store`
<pre><code class="api-signature language-rust ignore">pub fn push_local_store&lt;T: <a href="/api/pax-runtime-api/store.md#store">Store</a>&gt;(&amp;self, store: T)</code></pre>

Push component-local state onto the runtime stack for descendants to find.

##### `screenshot`
<pre><code class="api-signature language-rust ignore">pub fn screenshot(&amp;self, id: u32)</code></pre>

Request a screenshot capture from the chassis, keyed by caller-provided id.

##### `set_cursor`
<pre><code class="api-signature language-rust ignore">pub fn set_cursor(&amp;self, cursor: <a href="/api/pax-runtime-api/cursor.md#cursorstyle">CursorStyle</a>)</code></pre>

Ask the chassis to display the requested cursor over the app surface.

##### `subscribe`
<pre><code class="api-signature language-rust ignore">pub fn subscribe(&amp;self, dependencies: &amp;[UntypedProperty], f: impl Fn() + &#39;static)</code></pre>

Attach a dependency subscription whose callback runs when any dependency dirties.
