# host
<!-- summary: Native application entry and Apple view embedding. -->
<!-- tags: api, pax-kit -->

Native application entry and Apple view embedding.
Enable the `host` feature in the executable, normally through
`standalone = ["pax-kit/host"]` in the application's Cargo manifest.
`designtime` enables runtime inspection and template changes; it does not
start a development server. Production packaging excludes designtime.

```ignore
fn main() -> Result<(), pax_kit::host::Error> {
    pax_kit::init!()?.run()
}
```
Native application entry and embedding through the Pax facade.

`init!` and `init_with!` initialize in the calling process. They do not parse
application arguments or start build tools, development servers, or simulators.
Use `pax-cli` to package applications and run development sessions.

## Structs
### `Application`
A prepared native application in the calling process.

Initialization invokes the selected root initializer on the main thread.
Consume this value to run a standalone application. On Apple targets it can
also mount in a caller-owned host view; the Windows/Linux facade is standalone-only.

#### Implementations
##### `allow_inline_session_directory`
<pre><code class="api-signature language-rust ignore">pub fn allow_inline_session_directory(&amp;self, directory: &amp;<a href="../../api/pax-std/drawing/path.md#path">Path</a>) -&gt; Result&lt;(), <a href="../../api/pax-host/index.md#error">Error</a>&gt;</code></pre>

Authorize a private directory of CLI guest descriptors before running the host.
Each `InlineFrame` can then select a prepared macOS guest by its descriptor name.

##### `mount`
<pre><code class="api-signature language-rust ignore">pub fn mount(self, host: <a href="../../api/pax-host/native.md#viewhost">ViewHost</a>&lt;&#39;_&gt;) -&gt; Result&lt;<a href="../../api/pax-host/native.md#mountedapp">MountedApp</a>&lt;&#39;_&gt;, <a href="../../api/pax-host/index.md#error">Error</a>&gt;</code></pre>

Mount an explicitly initialized native application in a caller-owned view.

##### `register_inline_app`
<pre><code class="api-signature language-rust ignore">pub fn register_inline_app(&amp;self, name: &amp;str, bundle: &amp;<a href="../../api/pax-std/drawing/path.md#path">Path</a>) -&gt; Result&lt;(), <a href="../../api/pax-host/index.md#error">Error</a>&gt;</code></pre>

Register a trusted, independently built Pax macOS bundle before running or mounting.
An `<InlineFrame session="project" />` uses the matching name and owns the guest's lifetime.

##### `run`
<pre><code class="api-signature language-rust ignore">pub fn run(self) -&gt; Result&lt;(), <a href="../../api/pax-host/index.md#error">Error</a>&gt;</code></pre>

Enter the native application loop in the calling process.

##### `run_with`
<pre><code class="api-signature language-rust ignore">pub fn run_with(self, options: <a href="../../api/pax-host/native.md#windowoptions">WindowOptions</a>) -&gt; Result&lt;(), <a href="../../api/pax-host/index.md#error">Error</a>&gt;</code></pre>

Run a native desktop application with explicit window options.
