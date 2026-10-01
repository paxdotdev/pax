# application
<!-- summary: Application configuration and services, independent of any async executor. -->
<!-- tags: api, pax-runtime-api -->

Application configuration and services, independent of any async executor.

## Structs
### `AppBuilder`
Configure immutable, application-wide service registrations.

#### Properties
##### `target`
Type: [`TargetInfo`](../../api/pax-runtime-api/platform.md#targetinfo)

Target facts available before any node or root state exists.

##### `async_limits`
Type: [`AsyncLimits`](../../api/pax-runtime-api/async_runtime.md#asynclimits)

Bounded task and UI delivery resources for this application.

##### `shutdown_deadline`
Type: [`Duration`](../../api/pax-runtime-api/animation.md#duration)

Time before the host reports unfinished teardown. Dependencies remain
retained after expiry; this is not a cancellation guarantee.

#### Implementations
##### `id`
<pre><code class="api-signature language-rust ignore">pub fn id(&amp;self) -&gt; u64</code></pre>

Process-local identity allocated before configuration; never serialized.

##### `provide`
<pre><code class="api-signature language-rust ignore">pub fn provide&lt;T: &#39;static&gt;(&amp;mut self, service: T) -&gt; Result&lt;(), <a href="../../api/pax-runtime-api/application.md#serviceerror">ServiceError</a>&gt;</code></pre>

Register a plain service. Its destructor runs on the UI thread; resources
that can wait during destruction must not be registered as plain services.

##### `provide_managed`
<pre><code class="api-signature language-rust ignore">pub fn provide_managed&lt;T: <a href="../../api/pax-runtime-api/application_shutdown.md#managedservice">ManagedService</a>&gt;(&amp;mut self, service: T) -&gt; Result&lt;(), <a href="../../api/pax-runtime-api/application.md#serviceerror">ServiceError</a>&gt;</code></pre>

Register a service that can acknowledge asynchronous cleanup. Register
dependencies before dependents; teardown runs in reverse order.

---

### `AppContext`
Weak access to the application that owns a node. Cloning this context does
not keep an application active. Service handles and this context stay local.

#### Implementations
##### `async_scope`
<pre><code class="api-signature language-rust ignore">pub fn async_scope(&amp;self) -&gt; Result&lt;<a href="../../api/pax-runtime-api/async_runtime.md#asyncscope">AsyncScope</a>, <a href="../../api/pax-runtime-api/async_runtime.md#asyncerror">AsyncError</a>&gt;</code></pre>

Application lifetime, independent of individual mounted views.

##### `id`
<pre><code class="api-signature language-rust ignore">pub fn id(&amp;self) -&gt; Option&lt;u64&gt;</code></pre>

Process-local identity of this application instance while retained.

##### `phase`
<pre><code class="api-signature language-rust ignore">pub fn phase(&amp;self) -&gt; <a href="../../api/pax-runtime-api/application.md#appphase">AppPhase</a></code></pre>

Current lifecycle state; a released owner reports Closed.

##### `service`
<pre><code class="api-signature language-rust ignore">pub fn service&lt;T: &#39;static&gt;(&amp;self) -&gt; Result&lt;Rc&lt;T&gt;, <a href="../../api/pax-runtime-api/application.md#serviceerror">ServiceError</a>&gt;</code></pre>

Look up a registered concrete service without extending app authority.

##### `set_task_error_sink`
<pre><code class="api-signature language-rust ignore">pub fn set_task_error_sink(&amp;self, sink: impl Fn(String) + &#39;static) -&gt; Result&lt;(), <a href="../../api/pax-runtime-api/async_runtime.md#asyncerror">AsyncError</a>&gt;</code></pre>

Observe unhandled task panics on the UI thread, including dropped controls.
Reporting continues while managed shutdown retains the application. The
sink must not touch detached UI state once the phase is Closing.

##### `target`
<pre><code class="api-signature language-rust ignore">pub fn target(&amp;self) -&gt; Result&lt;<a href="../../api/pax-runtime-api/platform.md#targetinfo">TargetInfo</a>, <a href="../../api/pax-runtime-api/application.md#serviceerror">ServiceError</a>&gt;</code></pre>

Target facts selected by the owning host.

---

### `EmptyApplication`
The empty configuration used when the root omits the application attribute.

## Enums
### `AppPhase`
The activation and teardown state of one cartridge instance.

#### Variants
##### `Preparing`
##### `Active`
##### `Closing`
##### `Closed`
---

### `ServiceError`
A service lookup or registration error.

#### Variants
##### `Duplicate`(&'`static` `str`)
##### `Missing`(&'`static` `str`)
##### `Closed`
---

### `StopReason`
Why an application is leaving its active host.

#### Variants
##### `HostClosed`
##### `Replaced`
##### `PreparationDiscarded`
## Traits
### `Application`
Hooks selected by `#[application(Type)]` on the active root component.
Configuration runs before root defaults. Hooks run on the UI thread and
must not block it on network work or executor shutdown.

## Type Aliases
### `AppResult`
A fallible application configuration result.
