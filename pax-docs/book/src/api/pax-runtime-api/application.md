# application
<!-- summary: Root initialization and application lifetime, independent of any async executor. -->
<!-- tags: api, pax-runtime-api -->

Root initialization and application lifetime, independent of any async executor.

## Structs
### `AppClosed`
Completion of retained application cleanup. This observer does not own the
application and never blocks the UI thread.

---

### `AppContext`
Weak access to the application that owns a node. Cloning this context does
not keep an application active. This context stays local and provides no dependency lookup.

#### Implementations
##### `async_scope`
<pre><code class="api-signature language-rust ignore">pub fn async_scope(&amp;self) -&gt; Result&lt;<a href="../../api/pax-runtime-api/async_runtime.md#asyncscope">AsyncScope</a>, <a href="../../api/pax-runtime-api/async_runtime.md#asyncerror">AsyncError</a>&gt;</code></pre>

Application lifetime, independent of individual mounted views.

##### `closed`
<pre><code class="api-signature language-rust ignore">pub fn closed(&amp;self) -&gt; <a href="../../api/pax-runtime-api/application.md#appclosed">AppClosed</a></code></pre>

Observe disposal completion without driving cleanup. The host's wake
scheduler keeps cleanup progressing even when this future is not polled.

##### `id`
<pre><code class="api-signature language-rust ignore">pub fn id(&amp;self) -&gt; Option&lt;u64&gt;</code></pre>

Process-local identity of this application instance while retained.

##### `phase`
<pre><code class="api-signature language-rust ignore">pub fn phase(&amp;self) -&gt; <a href="../../api/pax-runtime-api/application.md#appphase">AppPhase</a></code></pre>

Current lifecycle state; a released owner reports Closed.

##### `set_task_error_sink`
<pre><code class="api-signature language-rust ignore">pub fn set_task_error_sink(&amp;self, sink: impl Fn(String) + &#39;static) -&gt; Result&lt;(), <a href="../../api/pax-runtime-api/async_runtime.md#asyncerror">AsyncError</a>&gt;</code></pre>

Observe unhandled task panics on the UI thread, including dropped controls.
Reporting continues while managed shutdown retains the application. The
sink must not touch detached UI state once the phase is Closing.

##### `target`
<pre><code class="api-signature language-rust ignore">pub fn target(&amp;self) -&gt; Result&lt;<a href="../../api/pax-runtime-api/platform.md#targetinfo">TargetInfo</a>, <a href="../../api/pax-runtime-api/async_runtime.md#asyncerror">AsyncError</a>&gt;</code></pre>

Target facts selected by the owning host.

---

### `AppOptions`
Resource limits and cleanup diagnostics for one application instance.
These options never construct an executor or register application services.

#### Properties
##### `async_limits`
Type: [`AsyncLimits`](../../api/pax-runtime-api/async_runtime.md#asynclimits)

##### `startup_data`
Type: `Vec`<`u8`>

Opaque application-defined bytes, copied by the host before preparation.
At most [`HOST_MESSAGE_LIMIT`] bytes. Pax does not choose their encoding.

##### `shutdown_deadline`
Type: [`Duration`](../../api/pax-runtime-api/animation.md#duration)

Diagnostic deadline only; unfinished cleanup retains its dependencies.

---

### `InitContext`
Context supplied to the ordinary root initializer on the UI owner thread.
The application's property graph is entered for the duration of the call.
Construct the root promptly, then use scoped work for asynchronous loading.

#### Properties
##### `target`
Type: [`TargetInfo`](../../api/pax-runtime-api/platform.md#targetinfo)

#### Implementations
##### `lifetime`
<pre><code class="api-signature language-rust ignore">pub fn lifetime(&amp;self) -&gt; &amp;<a href="../../api/pax-runtime-api/application.md#appcontext">AppContext</a></code></pre>

Cancellation and retained cleanup boundary for explicitly owned executors.

##### `on_host_request`
<pre><code class="api-signature language-rust ignore">pub fn on_host_request(&amp;self, handler: impl Fn(Vec&lt;u8&gt;) -&gt; <a href="../../api/pax-runtime-api/application.md#appresult">AppResult</a>&lt;Vec&lt;u8&gt;&gt; + &#39;static) -&gt; <a href="../../api/pax-runtime-api/application.md#appresult">AppResult</a></code></pre>

Install the application's one host request handler during initialization.
Requests run on the UI owner through the scoped delivery queue after
activation. Return promptly; start scoped async work for longer operations.
Requests and replies are bounded byte buffers with an app-defined encoding.
Closing cancels queued requests. The handler is released before cleanup.

##### `startup_data`
<pre><code class="api-signature language-rust ignore">pub fn startup_data(&amp;self) -&gt; &amp;[u8]</code></pre>

Read application-defined startup bytes. Decode and validate them in the
initializer; the host never receives access to the resulting Rust values.

## Enums
### `AppPhase`
The activation and teardown state of one cartridge instance.

#### Variants
##### `Preparing`
##### `Active`
##### `Closing`
##### `Closed`
---

### `StopReason`
Why an application is leaving its active host.

#### Variants
##### `HostClosed`
##### `Replaced`
##### `PreparationDiscarded`
## Type Aliases
### `AppResult`
A fallible application configuration result.

## Constants
### `HOST_MESSAGE_LIMIT`
Maximum encoded startup, request or reply size across a host boundary.
