# pax-tokio
<!-- summary: Optional Tokio integration for native Pax applications, without a custom main. -->
<!-- tags: api, pax-tokio -->

Optional Tokio integration for native Pax applications, without a custom main.

Register [`TokioService`] with `AppBuilder::provide_managed`. Tasks use an
explicit runtime handle and a Pax lifetime. No Tokio context is entered around
UI callbacks. Browser applications use their browser executor instead.

## Structs
### `BoundTasks`
UI-local bound executor. Each spawn creates an independently cancellable
operation below the owner. Keep a component-bound helper from on_mount when
later event contexts belong to controls inside that component.

#### Implementations
##### `spawn`
<pre><code class="api-signature language-rust ignore">pub fn spawn(&amp;self, future: impl Future + Send + &#39;static) -&gt; Result&lt;<a href="../../api/pax-runtime-api/async_runtime.md#taskcontrol">TaskControl</a>, <a href="../../api/pax-runtime-api/async_runtime.md#asyncerror">AsyncError</a>&gt;</code></pre>

Monitor ordinary work. Raw Property clones captured here remain writable
after cancellation; use spawn_into for revocable result publication.

##### `spawn_into`
<pre><code class="api-signature language-rust ignore">pub fn spawn_into&lt;T: SharedPropertyValue&gt;(&amp;self, target: &amp;Property&lt;T&gt;, future: impl Future + Send + &#39;static) -&gt; Result&lt;<a href="../../api/pax-runtime-api/async_runtime.md#taskcontrol">TaskControl</a>, <a href="../../api/pax-runtime-api/async_runtime.md#asyncerror">AsyncError</a>&gt;</code></pre>

Publish the returned value only if this task's operation is still live.
The future receives no raw clone of the target.

##### `spawn_with_completion`
<pre><code class="api-signature language-rust ignore">pub fn spawn_with_completion&lt;T: Send + &#39;static&gt;(&amp;self, future: impl Future + Send + &#39;static, callback: impl FnOnce(<a href="../../api/pax-runtime-api/async_runtime.md#taskoutcome">TaskOutcome</a>&lt;T&gt;) + &#39;static) -&gt; Result&lt;<a href="../../api/pax-runtime-api/async_runtime.md#taskcontrol">TaskControl</a>, <a href="../../api/pax-runtime-api/async_runtime.md#asyncerror">AsyncError</a>&gt;</code></pre>

Return a result or task panic to a UI-local closure, never running that
closure inline on the executor. Cancellation suppresses queued delivery.

---

### `TokioService`
Application-owned Tokio client. Owned mode holds the runtime on a dedicated
owner thread so even fallback destruction never waits on the UI thread.
Borrowed mode never shuts down the external runtime; its owner must keep it
alive and continuously driven, including when it has current-thread flavor.

#### Implementations
##### `borrowed`
<pre><code class="api-signature language-rust ignore">pub fn borrowed(handle: Handle) -&gt; Self</code></pre>

Borrow an explicitly supplied, continuously driven runtime. A Handle
alone does not keep the Runtime alive and does not drive current-thread I/O.

##### `for_node`
<pre><code class="api-signature language-rust ignore">pub fn for_node(&amp;self, node: &amp;impl <a href="../../api/pax-runtime-api/async_runtime.md#asyncscopesource">AsyncScopeSource</a>) -&gt; Result&lt;BoundTasks, <a href="../../api/pax-runtime-api/async_runtime.md#asyncerror">AsyncError</a>&gt;</code></pre>

Uses this exact node's context, not the component owning a handler's self.

##### `handle`
<pre><code class="api-signature language-rust ignore">pub fn handle(&amp;self) -&gt; Result&lt;Handle, <a href="../../api/pax-runtime-api/async_runtime.md#asyncerror">AsyncError</a>&gt;</code></pre>

Advanced untracked work: raw tasks are neither staged nor cancelled by
Pax scopes. Prefer the bound helpers for component/application work.

##### `owned`
<pre><code class="api-signature language-rust ignore">pub fn owned(runtime: Runtime) -&gt; <a href="../../api/pax-runtime-api/application.md#appresult">AppResult</a>&lt;Self&gt;</code></pre>

Own a multithread runtime. A current-thread runtime needs a continuously
driven service thread and the borrowed-handle constructor instead.
