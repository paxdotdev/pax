# loading
<!-- summary: Read-only state and actions for an application loading component. -->
<!-- tags: api, pax-runtime-api -->

Read-only state and actions for an application loading component.

Download progress counts decoded application-module bytes, not compressed
network traffic. Transfer completion does not imply presentation readiness.

## Structs
### `LoadingContext`
Loader-scoped, UI-thread handle returned by `NodeContext::loading()`.
It does not retain the host or expose its writable state property.

#### Implementations
##### `cancel`
<pre><code class="api-signature language-rust ignore">pub fn cancel(&amp;self) -&gt; bool</code></pre>

Request cancellation without re-entering the host from an event handler.

##### `observe`
<pre><code class="api-signature language-rust ignore">pub fn observe(&amp;self) -&gt; LocalProperty&lt;Option&lt;<a href="../../api/pax-runtime-api/loading.md#loadingstate">LoadingState</a>&gt;&gt;</code></pre>

Observe state through a one-way property suitable for `ctx.subscribe`.
Writes to the returned property never mutate the host's loading state.
The observation becomes `None` when the host closes the experience.

##### `retry`
<pre><code class="api-signature language-rust ignore">pub fn retry(&amp;self) -&gt; bool</code></pre>

Request a supported retry. Returns false after close, when unsupported,
or while another action is already queued. The host services it later.

##### `snapshot`
<pre><code class="api-signature language-rust ignore">pub fn snapshot(&amp;self) -&gt; Option&lt;<a href="../../api/pax-runtime-api/loading.md#loadingstate">LoadingState</a>&gt;</code></pre>

Read the current snapshot; `None` means the loading experience has closed.

---

### `LoadingDownload`
Application-module transfer counters. Unknown totals remain `None`.

#### Properties
##### `received_bytes`
Type: `u64`

Decoded bytes received during the current attempt.

##### `total_bytes`
Type: `Option`<`u64`>

Expected decoded size, when the build manifest or transport provides it.

#### Implementations
##### `fraction`
<pre><code class="api-signature language-rust ignore">pub fn fraction(&amp;self) -&gt; Option&lt;f64&gt;</code></pre>

Download fraction in 0..=1, or `None` without a positive denominator.
This is not an estimate of compilation, mounting or rendering work.

---

### `LoadingState`
A snapshot of startup state. Editing a snapshot never changes the host.

#### Properties
##### `phase`
Type: [`LoadingPhase`](../../api/pax-runtime-api/loading.md#loadingphase)

Current startup stage.

##### `download`
Type: [`LoadingDownload`](../../api/pax-runtime-api/loading.md#loadingdownload)

Transfer counters, independent of preparation/readiness.

##### `error`
Type: `Option`<`String`>

Human-readable failure information, when startup has failed.

##### `can_retry`
Type: `bool`

Whether the host can safely retry this attempt.

##### `can_cancel`
Type: `bool`

Whether the host currently accepts cancellation.

## Enums
### `LoadingPhase`
Stage of one application startup attempt.

#### Variants
##### `Downloading`
Fetching and compiling the deferred application module.

##### `Preparing`
Constructing, mounting, measuring and preparing the application.

##### `Ready`
The application's first presentation is ready.

##### `Revealing`
The loader is exiting over the prepared application; input remains gated.

##### `Active`
The application has been presented and activated.

##### `Failed`
Startup failed; consult the error and retry capability.

##### `Cancelled`
This startup attempt was cancelled.

#### Implementations
##### `as_str`
<pre><code class="api-signature language-rust ignore">pub fn as_str(self) -&gt; &amp;&#39;static str</code></pre>

Stable lowercase spelling exposed as `@loading.phase` in Pax.
