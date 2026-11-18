# clipboard
<!-- summary: Plain-text clipboard writes with chassis-owned access and scoped completion. -->
<!-- tags: api, pax-runtime-api -->

Plain-text clipboard writes with chassis-owned access and scoped completion.

## Structs
### `Clipboard`
Per-engine clipboard adapter. This is not a process-global clipboard cache.

Chassis adapters start the platform request synchronously to preserve browser
user activation, then report its actual completion. No clipboard reads occur.

#### Implementations
##### `new`
<pre><code class="api-signature language-rust ignore">pub fn new(writer: impl Fn(String, <a href="../../api/pax-runtime-api/clipboard.md#clipboardreply">ClipboardReply</a>) + &#39;static) -&gt; Self</code></pre>

Construct a chassis adapter; an unconfigured clipboard reports Unsupported.

##### `write_text`
<pre><code class="api-signature language-rust ignore">pub fn write_text(&amp;self, scope: &amp;<a href="../../api/pax-runtime-api/async_runtime.md#asyncscope">AsyncScope</a>, text: String, callback: impl FnOnce(Result&lt;(), <a href="../../api/pax-runtime-api/clipboard.md#clipboarderror">ClipboardError</a>&gt;) + &#39;static) -&gt; Result&lt;(), <a href="../../api/pax-runtime-api/async_runtime.md#asyncerror">AsyncError</a>&gt;</code></pre>

Start a write now and deliver its result on the UI thread while `scope`
remains alive. `Ok(())` means accepted, not copied. Closing the scope
suppresses the callback, but cannot undo a platform write already started.

## Enums
### `ClipboardError`
Why a plain-text clipboard write did not complete.

#### Variants
##### `Unsupported`
The chassis has no clipboard adapter, or the platform API is unavailable.

##### `NotAllowed`
The platform rejected access (for example, permission or user activation).

##### `Failed`
The platform reported another write failure.

## Type Aliases
### `ClipboardReply`
A chassis must call this exactly once after writing or failing. It may call
synchronously; user callbacks are always deferred through the UI scope.
