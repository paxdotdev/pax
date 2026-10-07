# native
<!-- summary: Apple process entry, view ownership and asynchronous mount observations. -->
<!-- tags: api, pax-host -->

Apple process entry, view ownership and asynchronous mount observations.

## Structs
### `Completion`
A readiness/cleanup observer. It may be dropped without cancelling cleanup.

---

### `MountedApp`
An owned single-region mount. Drop or `close` removes only Pax's child
hierarchy and revokes its work. Cleanup continues through host wakeups even
if every observer is dropped or the region is hidden.

#### Implementations
##### `close`
<pre><code class="api-signature language-rust ignore">pub fn close(&amp;self)</code></pre>

Start close without blocking the host thread.

##### `closed`
<pre><code class="api-signature language-rust ignore">pub fn closed(&amp;self) -&gt; Completion</code></pre>

Observe cleanup acknowledgement. Polling does not drive shutdown.

##### `ready`
<pre><code class="api-signature language-rust ignore">pub fn ready(&amp;self) -&gt; Completion</code></pre>

Observe host readiness, including the property wake route and viewport.

##### `request`
<pre><code class="api-signature language-rust ignore">pub fn request(&amp;self, bytes: &amp;[u8]) -&gt; <a href="../../api/pax-host/native.md#request">Request</a></code></pre>

Send an application-defined request and observe its reply without
blocking the UI thread. Each buffer is limited to 64 KiB and copied
across the boundary. Close rejects queued requests; dropping this future
discards observation without cancelling already accepted work.

---

### `PreparedApp`
A prepared cartridge, confined to the platform's main thread. Consume it
once with `run` or `mount`. Dropping it starts retained asynchronous cleanup.

#### Implementations
##### `mount`
<pre><code class="api-signature language-rust ignore">pub fn mount&lt;&#39;a&gt;(self, host: <a href="../../api/pax-host/native.md#viewhost">ViewHost</a>&lt;&#39;a&gt;) -&gt; Result&lt;<a href="../../api/pax-host/native.md#mountedapp">MountedApp</a>&lt;&#39;a&gt;, <a href="../../api/pax-host/index.md#error">Error</a>&gt;</code></pre>

Attach the full Pax hierarchy, including native controls, to a region of
the caller's view. The caller continues to own its window and event loop.

##### `run`
<pre><code class="api-signature language-rust ignore">pub fn run(self) -&gt; Result&lt;(), <a href="../../api/pax-host/index.md#error">Error</a>&gt;</code></pre>

Enter the platform's standalone SwiftUI application loop on the main
thread. On iOS this normally does not return. Configure process services
before calling this; cartridge-local services belong in its initializer.

##### `run_with`
<pre><code class="api-signature language-rust ignore">pub fn run_with(self, options: <a href="../../api/pax-host/native.md#windowoptions">WindowOptions</a>) -&gt; Result&lt;(), <a href="../../api/pax-host/index.md#error">Error</a>&gt;</code></pre>

Enter the standalone macOS loop with an explicit window policy.

---

### `Request`
Observation of one host request. Poll on the host's UI executor.

---

### `ViewHost`
A caller-owned native view and (on iOS) its containing view controller.
This borrow prevents the Rust wrapper from outliving the supplied owner.

#### Implementations
##### `from_raw`
<pre><code class="api-signature language-rust ignore">pub unsafe fn from_raw&lt;T&gt;(_: &amp;&#39;a mut T, view: NonNull&lt;c_void&gt;, controller: *mut c_void) -&gt; Self</code></pre>

Wrap a native container. `owner` must own the NSView on macOS, or both
UIView and UIViewController on iOS, for the entire mount lifetime.

# Safety
Pointers must have those Objective-C types, belong to the process main
thread, and remain valid until the mount is dropped. On iOS the controller
must own the container in its view hierarchy. Do not reparent a live mount.

---

### `WindowOptions`
Standalone macOS window policy, expressed in logical points. iOS/iPadOS
windows are controlled by their scene; embedded mounts use their host bounds.

#### Properties
##### `title`
Type: `Option`<`String`>

None preserves the application's bundle display name.

##### `size`
Type: (`f64`, `f64`)

##### `minimum_size`
Type: (`f64`, `f64`)

##### `resizable`
Type: `bool`

## Functions
### `init`
<pre><code class="api-signature language-rust ignore">pub fn init() -&gt; Result&lt;<a href="../../api/pax-host/native.md#preparedapp">PreparedApp</a>, <a href="../../api/pax-host/index.md#error">Error</a>&gt;</code></pre>

Prepare the bundled cartridge and invoke its typed root initializer once.
Must be called on the process main thread. No window or nested event loop is
created. Initializers should return a Loading root and stage asynchronous
work; staged work becomes eligible when the host is ready.

---

### `init_with`
<pre><code class="api-signature language-rust ignore">pub fn init_with(options: <a href="../../api/pax-host/index.md#initoptions">InitOptions</a>) -&gt; Result&lt;<a href="../../api/pax-host/native.md#preparedapp">PreparedApp</a>, <a href="../../api/pax-host/index.md#error">Error</a>&gt;</code></pre>

Prepare a cartridge with explicit encoded inputs. No Rust allocation or
object identity is shared with the cartridge. See [`init`] for lifecycle.
