# reference::example_host
<!-- summary: API docs for pax-std::reference::example_host. -->
<!-- tags: api, pax-std -->

## Structs
### `ExampleHost`
A display wrapper for documentation/demo examples.

`ExampleHost` renders every projected child with `slot()` and offers an
optional source drawer driven by an explicit `sources` manifest.
The code pane offers Solarized Dark (default) and Light through a moon/sun switch.
Theme state is local to the host unless `source_light_mode` is explicitly bound.
Set `inline_source_viewer=false` to present a separately mounted
[`ExampleSourceViewer`], binding its open, selection, and theme properties.

#### Properties
##### `title`
Type: `Property`<`String`>

Title shown in the source drawer.

##### `sources`
Type: `Property`<`Vec`<[`ExampleSource`](../../../api/pax-std/reference/example_host.md#examplesource)>>

Explicit source files related to the projected example subtree.

##### `selected_source`
Type: `Property`<`usize`>

Selected source index for the drawer tabs.

##### `drawer_open`
Type: `Property`<`bool`>

Whether the source drawer is visible.

##### `source_light_mode`
Type: `Property`<`bool`>

Use Solarized Light instead of Dark inside the code pane only.
The moon/sun switch edits this value; use `bind:` to share it deliberately.

##### `inline_source_viewer`
Type: `Property`<`bool`>

Present the source viewer beside the preview (default true).
When false, View Source still toggles `drawer_open`, but the caller must
mount an `ExampleSourceViewer` elsewhere, such as above a page Scroller.

#### Implementations
##### `on_pre_render`
<pre><code class="api-signature language-rust ignore">pub fn on_pre_render(&amp;mut self, ctx: &amp;<a href="../../../api/internal/pax-runtime/api.md#nodecontext">NodeContext</a>)</code></pre>

Keeps the animated drawer progress synchronized with external state writes.

##### `on_resize_mouse_down`
<pre><code class="api-signature language-rust ignore">pub fn on_resize_mouse_down(&amp;mut self, ctx: &amp;<a href="../../../api/internal/pax-runtime/api.md#nodecontext">NodeContext</a>, event: <a href="../../../api/pax-runtime-api/events.md#event">Event</a>&lt;<a href="../../../api/pax-runtime-api/events.md#mousedown">MouseDown</a>&gt;)</code></pre>

Starts dragging the source divider when pressed near the handle.

##### `on_resize_mouse_move`
<pre><code class="api-signature language-rust ignore">pub fn on_resize_mouse_move(&amp;mut self, ctx: &amp;<a href="../../../api/internal/pax-runtime/api.md#nodecontext">NodeContext</a>, event: <a href="../../../api/pax-runtime-api/events.md#event">Event</a>&lt;<a href="../../../api/pax-runtime-api/events.md#mousemove">MouseMove</a>&gt;)</code></pre>

Updates the fixed-sum preview/source split while dragging.

##### `on_resize_mouse_out`
<pre><code class="api-signature language-rust ignore">pub fn on_resize_mouse_out(&amp;mut self, ctx: &amp;<a href="../../../api/internal/pax-runtime/api.md#nodecontext">NodeContext</a>, _event: <a href="../../../api/pax-runtime-api/events.md#event">Event</a>&lt;<a href="../../../api/pax-runtime-api/events.md#mouseout">MouseOut</a>&gt;)</code></pre>

Restores the cursor after leaving the source divider.

##### `on_resize_mouse_over`
<pre><code class="api-signature language-rust ignore">pub fn on_resize_mouse_over(&amp;mut self, ctx: &amp;<a href="../../../api/internal/pax-runtime/api.md#nodecontext">NodeContext</a>, _event: <a href="../../../api/pax-runtime-api/events.md#event">Event</a>&lt;<a href="../../../api/pax-runtime-api/events.md#mouseover">MouseOver</a>&gt;)</code></pre>

Shows the platform resize cursor over the source divider.

##### `on_resize_mouse_up`
<pre><code class="api-signature language-rust ignore">pub fn on_resize_mouse_up(&amp;mut self, ctx: &amp;<a href="../../../api/internal/pax-runtime/api.md#nodecontext">NodeContext</a>, _event: <a href="../../../api/pax-runtime-api/events.md#event">Event</a>&lt;<a href="../../../api/pax-runtime-api/events.md#mouseup">MouseUp</a>&gt;)</code></pre>

Ends source divider dragging.

##### `on_split_mouse_down`
<pre><code class="api-signature language-rust ignore">pub fn on_split_mouse_down(&amp;mut self, ctx: &amp;<a href="../../../api/internal/pax-runtime/api.md#nodecontext">NodeContext</a>, _event: <a href="../../../api/pax-runtime-api/events.md#event">Event</a>&lt;<a href="../../../api/pax-runtime-api/events.md#mousedown">MouseDown</a>&gt;)</code></pre>

Starts dragging from the explicit source divider hit target.

##### `toggle_drawer`
<pre><code class="api-signature language-rust ignore">pub fn toggle_drawer(&amp;mut self, _ctx: &amp;<a href="../../../api/internal/pax-runtime/api.md#nodecontext">NodeContext</a>, _event: <a href="../../../api/pax-runtime-api/events.md#event">Event</a>&lt;<a href="../../../api/pax-runtime-api/events.md#click">Click</a>&gt;)</code></pre>

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

---

### `ExampleSourceViewer`
Reusable source panel with fixed black/white chrome and a Solarized code pane.

It fills the bounds supplied by its caller, not the global viewport. For a
full-screen presentation, mount it above (outside) the page Scroller and bind
`open`, `selected_source`, and `source_light_mode` to the owning state. The
caller controls mounting; the close control sets `open=false`.

#### Properties
##### `title`
Type: `Property`<`String`>

Wrapping title beside the controls; its measured height positions the source tabs.

##### `sources`
Type: `Property`<`Vec`<[`ExampleSource`](../../../api/pax-std/reference/example_host.md#examplesource)>>

Source files offered by this viewer.

##### `selected_source`
Type: `Property`<`usize`>

Selected source index; bind to preserve it across viewer remounts.

##### `open`
Type: `Property`<`bool`>

Close control writes false; the caller decides how to dismiss the viewer.

##### `source_light_mode`
Type: `Property`<`bool`>

False selects Solarized Dark; true selects Solarized Light. Chrome stays black/white.

#### Implementations
##### `source_background`
<pre><code class="api-signature language-rust ignore">pub fn source_background(light: bool) -&gt; <a href="../../../api/pax-runtime-api/color.md#color">Color</a></code></pre>

Code-pane background for a Solarized mode.

##### `source_foreground`
<pre><code class="api-signature language-rust ignore">pub fn source_foreground(light: bool) -&gt; <a href="../../../api/pax-runtime-api/color.md#color">Color</a></code></pre>

Code-pane body text for a Solarized mode.
