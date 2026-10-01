# async_runtime
<!-- summary: Executor-neutral task lifetimes and bounded delivery to the UI thread. -->
<!-- tags: api, pax-runtime-api -->

Executor-neutral task lifetimes and bounded delivery to the UI thread.

A scope is local authority. Only its producers, guarded publishers and task
reporters can cross threads. A successful send means accepted, not delivered.

## Structs
### `AsyncLimits`
Application resource limits. Counts do not bound the byte size of user values.

#### Properties
##### `scopes`
Type: `usize`

##### `tasks`
Type: `usize`

##### `publishers`
Type: `usize`

##### `endpoints`
Type: `usize`

##### `message_slots`
Type: `usize`

##### `channel_capacity`
Type: `usize`

##### `callbacks_per_turn`
Type: `usize`

##### `callback_time`
Type: [`Duration`](../../api/pax-runtime-api/animation.md#duration)

---

### `AsyncScope`
Local lifetime controller. Cancellation revokes descendants before requesting
task abortion. Keep a component's scope from its mount context when handlers
later receive contexts for child controls.

```compile_fail
use pax_runtime_api::{application::{ApplicationInstance, EmptyApplication}, TargetInfo};
let app = ApplicationInstance::prepare::<EmptyApplication>(TargetInfo::default()).unwrap();
let scope = app.context().async_scope().unwrap();
std::thread::spawn(move || scope.cancel());
```

#### Implementations
##### `child`
<pre><code class="api-signature language-rust ignore">pub fn child(&amp;self) -&gt; Result&lt;Self, <a href="../../api/pax-runtime-api/async_runtime.md#asyncerror">AsyncError</a>&gt;</code></pre>

Create an independent operation underneath this lifetime.

##### `register_task`
<pre><code class="api-signature language-rust ignore">pub fn register_task(&amp;self, start: impl FnOnce(<a href="../../api/pax-runtime-api/async_runtime.md#taskreporter">TaskReporter</a>) -&gt; Box&lt;dyn FnOnce()&gt; + &#39;static) -&gt; Result&lt;<a href="../../api/pax-runtime-api/async_runtime.md#taskcontrol">TaskControl</a>, <a href="../../api/pax-runtime-api/async_runtime.md#asyncerror">AsyncError</a>&gt;</code></pre>

Adapter entry point. `start` runs only after activation and returns a
nonblocking cancellation request. Retaining the control is optional.

##### `replace`
<pre><code class="api-signature language-rust ignore">pub fn replace&lt;T: SharedPropertyValue&gt;(&amp;self, previous: &amp;mut Option&lt;<a href="../../api/pax-runtime-api/async_runtime.md#asyncscope">AsyncScope</a>&gt;, property: &amp;Property&lt;T&gt;, initial: T) -&gt; Result&lt;<a href="../../api/pax-runtime-api/async_runtime.md#asyncscope">AsyncScope</a>, <a href="../../api/pax-runtime-api/async_runtime.md#asyncerror">AsyncError</a>&gt;</code></pre>

Revoke the previous request before publishing replacement state and
creating its new operation. Registration failure leaves the old work closed.

---

### `Closed`
A rejected publication retains the caller's value.

#### Properties
##### `0`
Type: `T`

---

### `Completion`
Single-use producer. Dropping it unused releases the callback on the UI turn.

---

### `GuardedPublisher`
A shared property writer whose commit is atomic with lifetime revocation.
Cloning this handle does not acquire another registration. Raw `Property`
clones remain deliberately unguarded.

---

### `SendFuture`
Capacity wait owned by one send attempt; dropping it unregisters its waker.

---

### `TaskControl`
UI-owned task control; dropping it does not detach work from its scope.

#### Implementations
##### `status_property`
<pre><code class="api-signature language-rust ignore">pub fn status_property(&amp;self) -&gt; Published&lt;<a href="../../api/pax-runtime-api/async_runtime.md#taskstatus">TaskStatus</a>&gt;</code></pre>

A snapshot property for reactive status displays; applications should not
mutate this adapter-owned state.

---

### `TaskReporter`
Transferable completion signal owned by an executor adapter. Drop means the
task stopped without normal completion, usually after an abort request.

---

### `UiSender`
Transferable producer for a bounded UI channel. Its callback stays local.

#### Implementations
##### `send`
<pre><code class="api-signature language-rust ignore">pub fn send(&amp;self, value: T) -&gt; <a href="../../api/pax-runtime-api/async_runtime.md#sendfuture">SendFuture</a>&lt;&#39;_, T&gt;</code></pre>

Wait for capacity without blocking the UI or an executor thread.

##### `try_send`
<pre><code class="api-signature language-rust ignore">pub fn try_send(&amp;self, value: T) -&gt; Result&lt;(), <a href="../../api/pax-runtime-api/async_runtime.md#trysenderror">TrySendError</a>&lt;T&gt;&gt;</code></pre>

Enqueue immediately, reporting Full or Closed without losing the value.

---

### `UiSubscription`
Keeps a UI channel subscribed. Drop closes it and releases its local callback.

## Enums
### `AsyncError`
New work was rejected before it could start or reserve resources.

#### Variants
##### `Closed`
##### `Limit`(&'`static` `str`)
##### `ZeroCapacity`
---

### `TaskOutcome`
The callback result keeps application `Result<T, E>` separate from a task panic.

#### Variants
##### `Completed`(`T`)
##### `Panicked`(`String`)
---

### `TaskStatus`
Observable task state. A cancellation request is not proof that work stopped.

#### Variants
##### `Staged`
##### `Running`
##### `CancelRequested`
##### `Completed`
##### `Cancelled`
##### `Panicked`(`String`)
---

### `TrySendError`
A nonblocking channel send either retains the value or accepts it exactly once.

#### Variants
##### `Full`(`T`)
##### `Closed`(`T`)
## Traits
### `AsyncScopeSource`
A local owner that can bind work to its lifetime without exposing a node to
an executor. Implemented by `AppContext` and runtime `NodeContext`.

## Functions
### `panic_message`
<pre><code class="api-signature language-rust ignore">pub fn panic_message(panic: Box&lt;dyn Any + Send&gt;) -&gt; String</code></pre>

Converts an unwinding panic into an observable task/callback failure.
