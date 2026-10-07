# host
<!-- summary: Development launch, native application entry, and Apple view embedding. -->
<!-- tags: api, pax-kit -->

Development launch, native application entry, and Apple view embedding.
Enable the `host` feature in the launcher, normally through
`standalone = ["pax-kit/host"]` in the application's Cargo manifest.
`designtime` adds workstation launch through the shared packager. Production
packaging excludes designtime; packaged native applications run directly.

```ignore
fn main() -> Result<(), pax_kit::host::Error> {
    pax_kit::init!()?.run()
}
```
Application launch and native embedding through the Pax facade.

`init!()?.run()` starts a development session when `designtime` is enabled
on a workstation. Packaged applications execute their native entry instead.
Use `init_with!` for explicit in-process initialization and embedding.

## Structs
### `Application`
A prepared native application or a workstation development session.

Preparing a development session does not initialize the target UI. Its root
initializer runs in the browser or native application after launch.

#### Implementations
##### `mount`
<pre><code class="api-signature language-rust ignore">pub fn mount(self, host: <a href="../../api/pax-host/native.md#viewhost">ViewHost</a>&lt;&#39;_&gt;) -&gt; Result&lt;<a href="../../api/pax-host/native.md#mountedapp">MountedApp</a>&lt;&#39;_&gt;, <a href="../../api/pax-host/index.md#error">Error</a>&gt;</code></pre>

Mount an explicitly initialized native application in a caller-owned view.
Use `init_with!` to ensure preparation happens in this process.

##### `run`
<pre><code class="api-signature language-rust ignore">pub fn run(self) -&gt; Result&lt;(), <a href="../../api/pax-host/index.md#error">Error</a>&gt;</code></pre>

Run the selected development session or enter the native application loop.

##### `run_with`
<pre><code class="api-signature language-rust ignore">pub fn run_with(self, options: <a href="../../api/pax-host/native.md#windowoptions">WindowOptions</a>) -&gt; Result&lt;(), <a href="../../api/pax-host/index.md#error">Error</a>&gt;</code></pre>

Run a native macOS application with explicit window options.
