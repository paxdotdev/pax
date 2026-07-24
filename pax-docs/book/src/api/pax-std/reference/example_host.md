# reference::example_host
<!-- summary: API docs for pax-std::reference::example_host. -->
<!-- tags: api, pax-std -->

## Structs
### `ExampleHost`
A display wrapper for documentation/demo examples.

`ExampleHost` renders every projected child with `slot()` and offers an
optional source drawer driven by an explicit `sources` manifest.

#### Properties
##### `title`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<`String`>

Title shown in the source drawer.

##### `sources`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<`Vec`<[`ExampleSource`](/api/pax-std/reference/example_host.md#examplesource)>>

Explicit source files related to the projected example subtree.

##### `selected_source`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<`usize`>

Selected source index for the drawer tabs.

##### `drawer_open`
Type: [`Property`](/api/pax-runtime-api/properties.md#property)<`bool`>

Whether the source drawer is visible.

#### Implementations
##### `on_pre_render`
<pre><code class="api-signature language-rust ignore">pub fn on_pre_render(&amp;mut self, _ctx: &amp;<a href="/api/internal/pax-runtime/api.md#nodecontext">NodeContext</a>)</code></pre>

Keeps the animated drawer progress synchronized with external state writes.

##### `on_resize_mouse_down`
<pre><code class="api-signature language-rust ignore">pub fn on_resize_mouse_down(&amp;mut self, ctx: &amp;<a href="/api/internal/pax-runtime/api.md#nodecontext">NodeContext</a>, event: <a href="/api/pax-runtime-api/events.md#event">Event</a>&lt;<a href="/api/pax-runtime-api/events.md#mousedown">MouseDown</a>&gt;)</code></pre>

Starts dragging the source divider when pressed near the handle.

##### `on_resize_mouse_move`
<pre><code class="api-signature language-rust ignore">pub fn on_resize_mouse_move(&amp;mut self, ctx: &amp;<a href="/api/internal/pax-runtime/api.md#nodecontext">NodeContext</a>, event: <a href="/api/pax-runtime-api/events.md#event">Event</a>&lt;<a href="/api/pax-runtime-api/events.md#mousemove">MouseMove</a>&gt;)</code></pre>

Updates the fixed-sum preview/source split while dragging.

##### `on_resize_mouse_out`
<pre><code class="api-signature language-rust ignore">pub fn on_resize_mouse_out(&amp;mut self, ctx: &amp;<a href="/api/internal/pax-runtime/api.md#nodecontext">NodeContext</a>, _event: <a href="/api/pax-runtime-api/events.md#event">Event</a>&lt;<a href="/api/pax-runtime-api/events.md#mouseout">MouseOut</a>&gt;)</code></pre>

Restores the cursor after leaving the source divider.

##### `on_resize_mouse_over`
<pre><code class="api-signature language-rust ignore">pub fn on_resize_mouse_over(&amp;mut self, ctx: &amp;<a href="/api/internal/pax-runtime/api.md#nodecontext">NodeContext</a>, _event: <a href="/api/pax-runtime-api/events.md#event">Event</a>&lt;<a href="/api/pax-runtime-api/events.md#mouseover">MouseOver</a>&gt;)</code></pre>

Shows the platform resize cursor over the source divider.

##### `on_resize_mouse_up`
<pre><code class="api-signature language-rust ignore">pub fn on_resize_mouse_up(&amp;mut self, ctx: &amp;<a href="/api/internal/pax-runtime/api.md#nodecontext">NodeContext</a>, _event: <a href="/api/pax-runtime-api/events.md#event">Event</a>&lt;<a href="/api/pax-runtime-api/events.md#mouseup">MouseUp</a>&gt;)</code></pre>

Ends source divider dragging.

##### `on_split_mouse_down`
<pre><code class="api-signature language-rust ignore">pub fn on_split_mouse_down(&amp;mut self, ctx: &amp;<a href="/api/internal/pax-runtime/api.md#nodecontext">NodeContext</a>, _event: <a href="/api/pax-runtime-api/events.md#event">Event</a>&lt;<a href="/api/pax-runtime-api/events.md#mousedown">MouseDown</a>&gt;)</code></pre>

Starts dragging from the explicit source divider hit target.

##### `toggle_drawer`
<pre><code class="api-signature language-rust ignore">pub fn toggle_drawer(&amp;mut self, _ctx: &amp;<a href="/api/internal/pax-runtime/api.md#nodecontext">NodeContext</a>, _event: <a href="/api/pax-runtime-api/events.md#event">Event</a>&lt;<a href="/api/pax-runtime-api/events.md#click">Click</a>&gt;)</code></pre>

Toggles the source drawer.

---

### `ExampleSource`
One source file shown by `ExampleHost`.

#### Properties
##### `label`
Type: `String`

File label shown in drawer tabs.

##### `language`
Type: `String`

Language label shown in the drawer.

##### `code`
Type: `String`

Source code contents.
