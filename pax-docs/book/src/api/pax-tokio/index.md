# pax-tokio
<!-- summary: Explicit Tokio ownership and scoped tasks for native Pax applications. -->
<!-- tags: api, pax-tokio -->

Explicit Tokio ownership and scoped tasks for native Pax applications.

Construct your runtime, then pass it and `cx.lifetime()` to [`Executor::owned`]
from the root initializer. The adapter uses that exact runtime and retains
cleanup until workers finish. UI callbacks do not implicitly enter Tokio.

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

### `Executor`
An explicitly supplied Tokio runtime bound to an application's lifetime.
Cloning this client shares the same runtime; dropping a client does not end
application authority. Closing the application cancels work and retains the
runtime on its owner thread until shutdown is acknowledged.

#### Implementations
##### `borrowed`
<pre><code class="api-signature language-rust ignore">pub fn borrowed(handle: Handle, lifetime: &amp;<a href="../../api/pax-runtime-api/application.md#appcontext">AppContext</a>) -&gt; <a href="../../api/pax-runtime-api/application.md#appresult">AppResult</a>&lt;Self&gt;</code></pre>

Borrow a continuously driven runtime. Its external owner must outlive
cleanup; this adapter never shuts down that runtime.

##### `for_application`
<pre><code class="api-signature language-rust ignore">pub fn for_application(&amp;self, app: &amp;<a href="../../api/pax-runtime-api/application.md#appcontext">AppContext</a>) -&gt; Result&lt;BoundTasks, <a href="../../api/pax-runtime-api/async_runtime.md#asyncerror">AsyncError</a>&gt;</code></pre>

Bind work to this application lifetime.

##### `for_node`
<pre><code class="api-signature language-rust ignore">pub fn for_node(&amp;self, node: &amp;impl <a href="../../api/pax-runtime-api/async_runtime.md#asyncscopesource">AsyncScopeSource</a>) -&gt; Result&lt;BoundTasks, <a href="../../api/pax-runtime-api/async_runtime.md#asyncerror">AsyncError</a>&gt;</code></pre>

Bind work to the exact node supplied by the caller.

##### `for_scope`
<pre><code class="api-signature language-rust ignore">pub fn for_scope(&amp;self, scope: <a href="../../api/pax-runtime-api/async_runtime.md#asyncscope">AsyncScope</a>) -&gt; Result&lt;BoundTasks, <a href="../../api/pax-runtime-api/async_runtime.md#asyncerror">AsyncError</a>&gt;</code></pre>

Bind work to an explicit operation or other scope.

##### `handle`
<pre><code class="api-signature language-rust ignore">pub fn handle(&amp;self) -&gt; Result&lt;Handle, <a href="../../api/pax-runtime-api/async_runtime.md#asyncerror">AsyncError</a>&gt;</code></pre>

Access the exact supplied runtime for advanced, untracked work.

##### `owned`
<pre><code class="api-signature language-rust ignore">pub fn owned(runtime: Runtime, lifetime: &amp;<a href="../../api/pax-runtime-api/application.md#appcontext">AppContext</a>) -&gt; <a href="../../api/pax-runtime-api/application.md#appresult">AppResult</a>&lt;Self&gt;</code></pre>

Adopt this exact multithread runtime without changing its configuration.
The lifetime must be open. Construct inside the cartridge initializer;
never pass a Rust Runtime or Handle through the native cartridge ABI.
