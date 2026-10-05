# pax-web-async
<!-- summary: Scoped browser futures with the same publication/delivery contracts as Pax's -->
<!-- tags: api, pax-web-async -->

Scoped browser futures with the same publication/delivery contracts as Pax's
native adapter. This crate does not provide Tokio networking in browser Wasm.

## Structs
### `BoundTasks`
Local bound executor. Dropping a task control does not detach its lifetime.

---

### `Executor`
Explicitly owned browser executor. Futures stay on the browser's thread;
component cancellation aborts their polling and revokes result authority.

#### Implementations
##### `new`
<pre><code class="api-signature language-rust ignore">pub fn new(lifetime: &amp;<a href="../../api/pax-runtime-api/application.md#appcontext">AppContext</a>) -&gt; Result&lt;Self, <a href="../../api/pax-runtime-api/async_runtime.md#asyncerror">AsyncError</a>&gt;</code></pre>

Attach local browser futures to the supplied application lifetime.
