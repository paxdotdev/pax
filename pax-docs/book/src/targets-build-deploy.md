# Targets, Build, and Deployment
<!-- summary: Choose a target, build a release, host a web application, and understand Apple packaging. -->
<!-- tags: targets, build, deployment, web, macos, ios, ipados, assets, metadata -->

A running development project is the starting point. To put it in someone
else's hands, choose a target, build its artifacts, and check those artifacts
in the environment where people will use them.

Pax targets web, macOS, iOS, and iPadOS. Web releases can be served as static
files. macOS has a release build path; distribution adds Apple's signing and
packaging steps. iOS and iPadOS support debug and release builds, including
local simulator and connected-device runs. [Apple distribution](#apple-distribution)
requires additional signing and delivery steps.

This chapter assumes you have completed [Getting Started](getting-started.md).
Commands run from your project's directory unless stated otherwise.
To select a project from another directory, use
`pax-cli build --path ./my-app --target web --release`. Its release feature
check and compilation both load that project's `.cargo/config.toml`, including
local dependency patches and registry configuration.

## Targets and workstations

The target is where the application runs. The workstation is where you compile it.

| Application target | CLI value | Development workstation | Required target tools |
| --- | --- | --- | --- |
| Web browser | `web` | macOS, Debian/Ubuntu Linux, or Windows | Rust, OS build tools, `wasm32-unknown-unknown`, and `wasm-pack` |
| macOS application | `macos` | macOS | Rust, full Xcode, and the selected Apple Rust targets |
| iPhone application | `ios` | macOS | Rust, full Xcode, iOS SDK/simulator, and the selected Apple Rust target |
| iPad application | `ipados` | macOS | The same Apple toolchain, with an iPad simulator or device |

`ipad` is also accepted as an alias for `ipados`. Linux and Windows support
the web development workflow; they are not native application targets in the
current CLI. See [workstation setup](getting-started.md#prepare-your-workstation)
for installation commands and [Apple preparation](#prepare-the-apple-toolchain)
for additional native requirements.

The published CLI includes its web interface bundle. Node.js/npm are needed
when rebuilding that interface from the Pax repository, rather than for the
ordinary installed-CLI workflow.

Target support does not promise identical rendering or controls everywhere.
[How Pax Runs](how-pax-runs.md#rendering-backends) explains backend selection;
feature chapters state their narrower target and backend limits. Include the
browsers, devices, native controls, and rendering effects your application
actually uses in its test plan.

## Entry and mounting

`pax_kit::host` contains the options, errors, and embedding types.
The common native entry is `pax_kit::init!()?.run()`: the macro selects this
package's UI library and initializes its root in the current native process.
`init!(my_ui)` selects a separate UI dependency; `init_with!` adds startup options.
`run` enters the platform application loop. Neither macro parses application
arguments or starts development tools, regardless of `designtime`.
Only copied build metadata crosses from that library into the launcher; the
engine and application objects stay in the separately loaded cartridge.
Depend on `pax-kit` and enable its `host` feature in the launcher.
It creates no async runtime. UI state and executor setup belong in the
initializer selected by `runnable!`. Browser applications use the library's `init_web` entrypoint instead. Shared Cargo features do not mount the UI or
link the native host into a cartridge.

Standalone Apple applications have two Rust targets: a library cartridge containing the root
component and its initializer, and an Apple executable with an ordinary Rust
`fn main`. The executable uses `pax_kit::host` to enter the SwiftUI host framework; the
cartridge remains separately linked. Use the launch macro to select the application library. Do not pass a `Root`,
`Property`, Tokio runtime, or Rust closure across its C boundary. Construct cartridge-owned resources in the
[typed root initializer](event-handling-rust.md#application-setup).

The generated example manifests make the launcher explicit:

```toml
[package]
default-run = "my-app"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
pax-kit = { version = "0.39.0" }

[features]
default = ["standalone", "designtime"]
standalone = ["pax-kit/host"]
designtime = ["pax-kit/designtime"]

[[bin]]
name = "my-app"
path = "src/main.rs"
required-features = ["standalone"]
```

The compilation target automatically selects the macOS, iOS, or web chassis.
`standalone` enables the native executable and host API. Turn it off with
`--no-default-features --lib` when building only the UI library. Dependencies
can use `default-features = false`; feature unification is safe because loading
and runtime ownership remain separate.

On macOS, `cargo run`, `cargo run -p my-app`, and the resulting executable run
the native application directly. Cargo builds the native host SDK before
execution. `designtime` enables runtime inspection and template changes; the CLI starts the server needed for those sessions.
The application does not depend on Pax's packager. Macro/compiler dependencies
run during compilation, not inside the native or browser application. Web
packaging compiles the Wasm and its manifest together, using the target's actual
cfgs, features, and profile, then reads that manifest to package routes and assets.
Apple packaging obtains its manifest during a target `cargo build` and reuses
the compiled library in its platform build. Neither needs a separate
`cargo check` compilation to extract the manifest.
If several standalone binaries exist, set `default-run`.
A host package may select a separate workspace UI library
through `[package.metadata.pax.cartridge]`; see
[workspace cartridge selection](components-composition.md#use-a-separate-ui-package).
The selected UI supplies component templates and assets; the host package owns
platform metadata, `public/`, and generated `.pax/` output.

```rust,ignore
fn main() -> Result<(), pax_kit::host::Error> {
    pax_kit::init!()?.run()
}
```

This is the complete bootstrap for the common native case. The same process
owns code before and after initialization, both in direct Cargo runs and in
packaged applications. Application arguments remain yours to parse. Use
`init_with!` for startup data or an explicit root selection, and `mount` for
embedding in a caller-owned view.

Put portable UI setup in the `runnable!` initializer, which runs in the target
application. Native `main` is not transported into a browser. Pax creates no
async runtime; each application retains its own executor policy.

The actual application `main` runs on all three Apple targets, including iOS and
iPadOS. `run` hands control to SwiftUI and normally never returns on mobile. Web
builds compile the UI library and JavaScript explicitly invokes its `init_web`;
importing the module does not mount it. Android and native Linux/Windows remain
unsupported. Those desktop workstations can run the web development tools.

On macOS, `init_with!(InitOptions::default())?.run_with(WindowOptions { ... })`
accepts window geometry and title. The function forms `pax_kit::init()` and
`init_with(options)` also remain explicit native APIs for hosts that already
package their cartridge. The macro forms identify the Cargo UI automatically.

When a library declares several runnable roots, select one explicitly:

```rust,ignore
let app = pax_kit::init_with!(pax_kit::host::InitOptions {
    entry: Some("crate::ui::App".into()),
    ..Default::default()
})?;
app.run()?;
```

Browser hosts select the same canonical name with
`Pax.init_web({ element, cartridgeUrl, entry: "crate::ui::App" })`.
An omitted name is accepted only when the library declares one distinct root.
An unknown or ambiguous name fails before running an initializer.

A runnable library can also supply components to another runnable library.
Reusing its components does not mount its application or call its initializer.
For web builds, Pax packaging exposes the selected cartridge as `init_web`;
dependency entrypoints remain distinct even when Cargo features are shared.

### Mount in an existing application

The host can supply one existing region: an `NSView` on macOS, a `UIView` plus
its owning `UIViewController` on iOS/iPadOS, or an `HTMLElement` in the browser.
Pax installs its normal child hierarchy, including native controls. It owns that
child and session; the caller retains its window, parent and event loop.

On Apple, call `pax_kit::init_with!(my_ui, pax_kit::host::InitOptions::default())`
and consume the prepared value with
`app.mount(view_host)`. `ViewHost::from_raw` is unsafe: its documented contract
requires valid platform objects, a retained owner borrowed for the mount's
lifetime, and the process main thread. An arbitrary GPU/window handle is not a
complete adapter. Swift/Objective-C hosts use the versioned C declarations in
`pax-host/include/pax_host.h`; see the repository's
`tests/src/host-embedding` fixture for containment and callback ownership.

An embedded Apple mount preserves the host's keyboard focus. Clicking or
tapping Pax's rendered content focuses its canvas; native text controls keep
their own input. See [Keyboard delivery](event-handling-rust.md#keyboard-delivery).
Rebuild the Apple host and cartridge together when updating Pax: cartridge ABI
3 supports selecting a declared runnable root, and preserves keyboard
default-prevention status from `pax_interrupt`. The host rejects older cartridges.
The host ABI is version 2, including copied launch configuration.

`mount.ready()` observes installation of the viewport and wake route.
`mount.close()` revokes work and removes only Pax's child. `mount.closed()`
observes cleanup acknowledgement. These futures must be driven by the caller's
UI executor; do not block the main thread waiting for them. Dropping the mount
starts close, and cleanup continues even if the observer is dropped. Keep the
parent/controller alive until the mount is dropped. Do not reparent a live mount.

A browser host loads the matching generated interface bundle and calls:

```html
<script src="pax-interface-web.js"></script>
<div id="ui" style="width:100%;height:600px"></div>
<script type="module">
  const cancellation = new AbortController();
  const app = await Pax.init_web({
    element: document.getElementById("ui"),
    cartridgeUrl: new URL("pax-cartridge", document.baseURI).href,
    signal: cancellation.signal,
  });
  // Promise resolution means mounted, not that application data has loaded.
  // On disposal: app.close(); await app.closed;
</script>
```

The container must belong to the current document and have an explicit usable
size. Pax follows its bounds, clips inside the region and routes input relative
to it. CSS transforms on the container or its ancestors are not supported.
`pageScroll` and `documentRouting` default to false for embedding; the standalone
page and compatibility `Pax.mount(element, cartridgeUrl)` entry enable them to
own page scrolling, history and metadata. With document
routing disabled, a bubbling `pax-navigate` event carries `{url, target}` for the
host to handle. Aborting `signal` cancels startup or closes an existing mount.
During bootstrap loading, cancellation rejects that invocation promptly. Other
callers sharing the same cartridge continue loading; the bootstrap fetch is
aborted when its last interested caller cancels. An immediate retry waits for
any abandoned Wasm initialization to settle before reusing or restarting it.
Startup failures reject the Promise and initiate retained cleanup. Failed native
preparation similarly returns an error; it can leave cleanup pending even though
no mount handle was returned. A retry is rejected while that cleanup is pending.

Wheel handlers are passive by default. An embedded host can pass
`wheelEvents: "cancellable"` to `Pax.init_web` when its application needs to
prevent native wheel scrolling; this overrides generated `wheel_events` metadata.
Cancellation can delay scrolling because the browser waits for JavaScript.
See [wheel input policy](scrolling-viewports.md#wheel-input-policy).

Native hosts support one live mount per runtime. A second prepare/mount is an
error, including while cleanup is pending. Await `closed` before replacing it.
On web, separate `init_web` calls own independent mounted regions and can share
the same cartridge's bootstrap download and initialized Wasm runtime.
Hidden/zero-size regions retain application state and cleanup scheduling.
Collapsing a native region does not prevent readiness or queued UI delivery;
its host skips drawable acquisition until the region has nonzero bounds. Closing
is distinct from suspension. Native controls retain their existing chassis
capabilities and restrictions; see [Accessibility and Native Controls](accessibility-native-controls.md).
No raw-surface-only or shared-wgpu-device adapter is provided.

### Data across the cartridge boundary

Use `pax_kit::init_with!(pax_kit::host::InitOptions { startup_data, ..Default::default() })` on Apple,
or `startupData: Uint8Array` in web options. The cartridge reads `InitContext::startup_data()` and
validates its own encoding. Inputs are copied before initialization; on web,
`init_web` validates and copies the buffer when called, before any download or
loader wait. Later mutations to the caller's buffer do not affect initialization
or a loading-screen retry. Empty data is the default; buffers are limited to
64 KiB.

An initializer can install one explicit request handler with
`cx.on_host_request(|bytes| -> AppResult<Vec<u8>> { ... })`. A mounted host calls
`mount.request(&bytes)` on Rust or `await app.request(bytes)` on web. Requests
enter the bounded application delivery queue and run on the UI owner after
activation. Replies are copied buffers, also limited to 64 KiB. Rejection,
handler errors and cancellation are observable; closing cancels queued requests.
Dropping a Rust reply future discards observation but does not undo an accepted
request. Native C reply buffers are borrowed only for the callback and must be
copied before it returns. Callbacks arrive asynchronously on the main thread.

This is a request/reply channel, not a resource registry or an unsolicited event
bus. Keep handlers short; they may update properties or start scoped work and
return an acknowledgement. The Async Workbench embedding fixtures demonstrate
UTF-8 startup data and `increment`/`read` requests. A Tokio `Handle` is a Rust
value and may only be borrowed within the same linkage domain, never encoded as
a pointer and passed through this interface.

## Run, build, and release

From an application package on macOS:

```sh
cargo run                  # native application; no development server
cargo build                # compile only; no packaging or server
```

In the repository, use `cargo run -p increment` or `cargo run --example increment`.
The package form compiles fewer unrelated example dependencies. Application
arguments follow Cargo's `--`; Pax initialization does not intercept them.
Native Linux and Windows hosting is not yet supported. Use the CLI's web target
on those workstations.

For a development session with template reload, serving, or simulator deployment:

```sh
pax-cli run --target web --port 8080
pax-cli run --target macos
pax-cli run --target ios        # iPhone simulator, on macOS
pax-cli run --target ipados     # iPad simulator, on macOS
pax-cli run --help
```

An explicit occupied web port fails rather than silently selecting another.
The CLI runs on the workstation; the native application's `main` runs in the
launched application process. Browser applications use `init_web` instead.

To package without launching, including production web output:

```sh
pax-cli build --target web --release
pax-cli build --target macos --release
```

Production packaging requires release artifacts without `designtime`. Pax omits
the starter's direct `designtime` default, preserves other application defaults,
and rejects aliases or dependencies that re-enable designtime in the target
runtime graph. Reusable UI dependencies should use `default-features = false`
and forward development features optionally. Build-time compiler dependencies
are permitted; runtime editing/replacement code is not part of the production
application.

The release feature check applies to both `runnable!` declarations and the legacy
`#[main]` shorthand; switching entrypoint syntax does not bypass it.

For a direct native Cargo build without development capabilities:

```sh
cargo run --no-default-features --features standalone
cargo build --release --no-default-features --features standalone
```

Add any other application features you require. Cargo's `--release` selects
optimized compilation; it does not package an application or remove default
features. Raw iOS Cargo builds can use `--target aarch64-apple-ios-sim`, but
producing/installing an app bundle requires CLI packaging. No Cargo runner is
configured by Pax.

For only the UI library, use `cargo build --lib --no-default-features` and add
the Rust `--target` you need. A raw WebAssembly build similarly needs JavaScript
bindings, its platform interface and assets before it can run in a browser.

The CLI manages local platform bundles, serving,
template reload, and mobile deployment:

```sh
pax-cli run --target web
pax-cli build --target web --release
pax-cli build --target macos --release
pax-cli build --target ios --release
```

Choose the command for your target. CLI `run` defaults to a debug build and
starts the target's development harness. CLI `build` also defaults to debug;
`--release` selects the optimized release path. iOS and iPadOS additionally accept `run --release` to
install and launch a local release. For web and macOS, use `build --release`.
For distribution, keep the complete generated bundle. Apple signing,
notarization, archive/export and store delivery remain additional steps;
`pax-cli build --release` does not perform an App Store submission.

You can work from another directory with `--path`:

```sh
pax-cli build --path path/to/my-project --target web --release
```

Release builds exclude the development service and hot-reload machinery.
Test the release itself: an application that works in a live development
session still needs a release check. Read [Debug and release](how-pax-runs.md#debug-and-release)
for the runtime differences, and [Developer Workflow](developer-workflow.md)
for reload configuration.

### Where the output goes

Cargo uses `target/debug/` or `target/release/`, with a target-triple directory
inserted when `--target` is specified. Workspace configuration and
`CARGO_TARGET_DIR` can change that location. The CLI's packaged output is
separate, beneath the application's `.pax/` directory:

| Build | Output |
| --- | --- |
| Web debug | `.pax/build/debug/web/` |
| Web release | `.pax/build/release/web/` |
| Web `--profiling` | `.pax/build/profiling/web/` |
| macOS debug | `.pax/build/debug/macos/app/Pax macOS (Development).app` |
| macOS release | `.pax/build/release/macos/app/Pax macOS (Release).app` |
| iOS debug | `.pax/build/debug/ios/app/Pax iOS (Development).app` |
| iPadOS debug | `.pax/build/debug/ipados/app/Pax iOS (Development).app` |
| iOS release | `.pax/build/release/ios/app/Pax iOS (Development).app` |
| iPadOS release | `.pax/build/release/ipados/app/Pax iOS (Development).app` |

iOS and iPadOS share the generated iOS host, including that bundle filename.
The mobile bundle filename retains “Development” even when built with the
Release configuration. The display name inside the app can be configured independently.

`--profiling` is web-only: it produces optimized output with Wasm names retained
for size analysis. It is separate from the normal release directory. See
[How Pax Runs](how-pax-runs.md) for performance investigation.

Build output is replaceable. Keep authored changes in project source, assets,
metadata, or an explicitly customized interface. If you use `pax-cli clean`,
it removes the project's entire `.pax/` directory, including builds and local
development artifacts. Save anything you need before cleaning.

## Build a web release

From the project root:

```sh
pax-cli build --target web --release
```

A successful build prints the output location. A typical directory contains:

```text
.pax/build/release/web/
├── index.html
├── pax-interface-web.js
├── pax-interface-web.css
├── pax-cartridge.js
├── pax-cartridge_bg.wasm
├── snippets/
├── assets/
└── … favicon, public files, and supporting files
```

Keep the directory together. The HTML loads the interface and cartridge, and
the cartridge depends on its matching Wasm and supporting files. Uploading
only `index.html` or only the `.wasm` file is insufficient.

### Preview the release locally

Serve the output over HTTP rather than opening `index.html` as a `file:` URL.
For example, if Python 3 is installed:

```sh
python3 -m http.server 8080 --bind 127.0.0.1 --directory .pax/build/release/web
```

On Windows, `py -3` can replace `python3`. Open
`http://127.0.0.1:8080/`, interact with the app, and inspect the browser console
and network requests. This server is for local review. It does not configure
production HTTPS, caching, or application-route fallback.

### Initial page background

Set `background_color = "#11110f"` under `[package.metadata.pax.web]` to match
an application's dark canvas from the first HTML paint. It accepts opaque
`#RGB` or `#RRGGBB` colors and applies to `html`, `body`, and `#mount` in both
debug and release builds. Without it, the host's existing styles apply.
This colors the web host; keep the Pax scene's background in sync. Rebuild
after changing metadata. No application mounting or lifecycle is delayed.

### Yield initial web mounting

Set `yield_mount = true` under `[package.metadata.pax.web]` to spread initial
engine work and native patches over browser tasks. The default is false. This
works with debug and release cartridges and does not change Apple targets.
Custom hosts can pass `yieldMount: true` or `false` to `Pax.init_web` to override
the generated metadata. Diagnostic URL overrides take precedence. See
[resumable startup](how-pax-runs.md#experimental-resumable-startup-on-web) for
presentation guarantees, input handling, and the optional early-visibility
experiment. Rebuild after changing metadata.

### Suspend an embedded web app

A custom web host can call `window.Pax.setSuspended(true)` to suspend frame
updates and drawing without unmounting the app. Call it with `false` to resume.
For a same-origin iframe, the host can access the API through
`iframe.contentWindow.Pax`; wait for the iframe's `load` event first.
Loading the iframe with `?pax_suspended=1` starts it suspended, including while
its Wasm is loading. The standalone URL needs no such parameter.

Use an `IntersectionObserver` on the iframe's stage and the host document's
`visibilitychange` event to resume only while the example is in view and the
host tab is visible. The docs use this arrangement. Properties and form state
remain mounted; this does not cancel application requests, freeze wall-clock
time, or pause arbitrary JavaScript timers. Elapsed-time-based animations may
catch up when frame updates resume.

## Assets and web public files

Use `assets/` for application media. Pax copies those assets into the target's
build. Images and fonts still need suitable source paths and loading behavior;
[Text, Fonts, and Images](text-fonts-images.md) owns those authoring details.
Remote resources remain network dependencies unless explicitly bundled or
vendored by the relevant target build path.

### Web public files

Create `public/` beside `Cargo.toml` for web files that should be served as
ordinary HTTP responses. Relative paths and bytes are preserved:

```text
public/ai.md             → /ai.md
public/robots.txt        → /robots.txt
public/.well-known/pax   → /.well-known/pax
public/guide/index.html  → /guide/
```

Those URLs assume deployment at the site's root. A build copies these files
into its web output. During `pax-cli run`, the server reads the source
`public/` directory directly, so edits, additions, and deletions become visible
after a browser refresh without restarting Pax.

Public files cannot overwrite generated or customized interface files.
`public/index.html` conflicts with the application's entry page; the top-level
`assets`, `snippets`, and `__reloads__` directories are reserved. Symbolic links
are rejected. Review everything placed here: these files are intended to be
public, including dot-prefixed files such as `.well-known` entries.
The hosting server determines response headers and content types; copying a
public file does not configure those policies.

Use `public/` for web-only documents and files such as `robots.txt`. It is not
packaged as an Apple application's asset directory. For titles, descriptions,
and social previews tied to application routes, use
[web route metadata](routing.md#web-route-metadata). The compiler generates
those entry documents from the route declarations. Avoid a public-file path
that conflicts with a generated route entry.

For navigation from Pax to these same-origin documents, declare their paths in
`[package.metadata.pax.web].server_owned_prefixes`. See
[Server-owned web paths](routing.md#server-owned-web-paths) for the complete
navigation and local-server contract. Production hosting must also serve those
paths directly and exclude them from any application fallback.

## Deploy web

A web build needs a static host. It does not need a running Pax CLI on the
server. For a first deployment, serve the app at the root of a domain or
subdomain, such as `https://app.example.com/`.

1. Build and test `.pax/build/release/web/` locally.
2. Publish that directory's **contents** as the site's document root, including
   `snippets/`, assets, and any dot-prefixed public files.
3. Enable HTTPS and correct content types on the host.
4. Open the deployed URL in a fresh browser session. Test interaction, media,
   and any direct route URLs before sharing it.

If a hosting service accepts a prebuilt directory, choose this web output as
its publish directory. Do not upload the project root, `Cargo.toml`, or the
whole `.pax/` directory. If a service builds from source, its build environment
also needs the [web prerequisites](#targets-and-workstations); a generic
JavaScript-only build image is not enough.

### A static-server configuration

For an existing NGINX installation, this is a minimal document-root example.
`/srv/pax/my-app` stands for a directory containing the complete release output;
adjust it and the MIME include path for your server. The enclosing `http` block
and public HTTPS configuration belong to the server administrator.

```nginx
server {
    listen 127.0.0.1:8080;
    server_name localhost;
    root /srv/pax/my-app;
    index index.html;
    include /etc/nginx/mime.types;
    add_header Cache-Control "no-cache";

    location / {
        try_files $uri $uri/ =404;
    }
}
```

Here, an existing file or directory is served; an unknown path returns 404.
Check the configuration with `nginx -t` before applying it. See NGINX's
[static-content guide](https://docs.nginx.com/nginx/admin-guide/web-server/serving-static-content/)
for how `root`, `index`, and `try_files` work. This example is a starting point
for a configured host, not a command to expose a development server publicly.

### Direct links and the base URL

An in-app navigation can work even when a fresh request to the same URL fails.
When someone opens `/teams/42/settings`, the server receives that path before
Pax starts. There are two separate requirements:

- The server must serve the app's entry HTML for application routes that have
  no corresponding static file.
- The HTML must still load scripts, Wasm, styles, and media from the correct
  build directory.

The web build prepares the entry documents' base URL from
`[package.metadata.pax.web].site_url`, using `/` when no site URL is supplied.
For a domain-root application, the generated HTML includes:

```html
<base href="/">
```

The default bootstrap uses `document.baseURI`, so a nested entry can load the
same bundle as the root entry. No interface ejection is needed for this
standard routing setup. HTML's
[`base` element](https://developer.mozilla.org/en-US/docs/Web/HTML/Reference/Elements/base)
also affects other relative document URLs, including fragment links; review
any custom interface HTML alongside the app. The metadata pass sets its base
as well. See [Routing](routing.md#web-route-metadata) for the declarations and
generated `route-metadata.json` and route-specific HTML files.

Then configure fallback for the application's route space. For example, an
NGINX app with routes beneath `/teams/` can add:

```nginx
location /teams/ {
    try_files $uri $uri/ /index.html;
}
```

Keep real public documents and assets ahead of application fallback. Missing
JS, CSS, Wasm, and media should return 404 rather than the app's HTML. A blanket
fallback can conceal a missing file behind a script or MIME error.

Serve generated route entries ahead of the fallback. Preserve the requested
URL rather than redirecting it to `/`; the app needs that path to choose its
content. The default route controls the in-app not-found screen, while the
server controls HTTP status codes.

A `site_url` containing `/my-app/` supplies that HTML base, but does not strip
the prefix from the pathname delivered to `Router`. The metadata matcher also
sees the browser pathname. Do not treat this setting as an end-to-end router
mount-prefix option: route definitions, generated entry locations, canonical
URLs, and host mapping all need to agree. Prefer a domain-root deployment for
the launch path, and verify those pieces separately before using a subdirectory.
See [Routing](routing.md) for nested scope, parameters, and navigation.

### Embedded examples

The default web interface supports an explicit query-backed routing mode for
examples hosted inside another site. Open its entry document with `pax_route=/`
(URL-encoded as `?pax_route=%2F`) to start at the application root. In this mode,
Pax loads bundle resources beside that entry document and stores subsequent
same-tab routes, including their query and fragment, inside `pax_route`.
Back, Forward, and reload keep the physical entry URL intact. The docs use this
mode for both embedded examples and their **Open standalone** links.

Omit the parameter for normal pathname routing. Query-backed examples do not
need a server fallback for each application route, but this is a preview/host
integration mode, not a replacement for the crawler-visible route entries
described above. A custom ejected web interface must preserve the default
bootstrap's embedded-base setup if it uses this mode.

### HTTPS, content types, and caching

Serve production apps over HTTPS. WebGPU requires a secure context; loopback
development URLs have special treatment. Backend availability still depends
on the browser and device. See [MDN's WebGPU reference](https://developer.mozilla.org/en-US/docs/Web/API/WebGPU_API).

Automatic renderer selection falls back to Vello CPU when WebGPU initialization
fails. Default CLI builds require WebAssembly SIMD, including Safari 16.4 or
newer. See [web build profiles](developer-workflow.md#web-build-profiles) for
compiler-flag overrides, and [renderer selection](how-pax-runs.md#rendering-backends)
for the CPU/GPU testing controls.

Return `.wasm` with `Content-Type: application/wasm`, and JavaScript/CSS with
their appropriate types. The Wasm MIME type enables streaming compilation.
[MDN documents this requirement](https://developer.mozilla.org/en-US/docs/WebAssembly/Reference/JavaScript_interface/instantiateStreaming_static).
Configure transfer compression on the host if desired: the CLI's gzip-size
report is a measurement, not proof that the server sends compressed responses.

Pax's main output filenames are stable across builds. For files updated at the
same URL, a conservative starting policy is `Cache-Control: no-cache`, allowing
storage but requiring revalidation. Reserve long-lived `immutable` caching for
versioned or content-addressed URLs whose contents will never change. See
[HTTP caching](https://developer.mozilla.org/en-US/docs/Web/HTTP/Guides/Caching).

Deploy matching HTML, JavaScript, Wasm, and assets together. Retain the previous
complete release for rollback, and test an upgrade with an already-open app
as well as a fresh load. A versioned asset layout can keep old and new clients
from fetching incompatible files; it requires an explicit URL/hosting plan,
since Pax does not automatically hash all of these filenames.

## Build for Apple platforms

### Prepare the Apple toolchain

Install full Xcode and the platform components you need. Command Line Tools
alone are enough for some web prerequisites, but do not provide the complete
native app/simulator workflow. Confirm the active installation:

```sh
xcode-select -p
xcodebuild -version
rustup target list --installed
```

Complete Xcode's initial setup and select the intended Xcode installation if
these commands report only a Command Line Tools directory. Install the Rust
targets needed for the build you plan to run:

| Build destination | Rust target |
| --- | --- |
| macOS on Apple silicon | `aarch64-apple-darwin` |
| macOS on Intel | `x86_64-apple-darwin` |
| iPhone or iPad device | `aarch64-apple-ios` |
| iOS/iPadOS simulator on Apple silicon | `aarch64-apple-ios-sim` |
| iOS/iPadOS simulator on Intel | `x86_64-apple-ios` |

For example, on an Apple-silicon Mac preparing for a macOS release and mobile
simulator/device development:

```sh
rustup target add aarch64-apple-darwin x86_64-apple-darwin
rustup target add aarch64-apple-ios-sim aarch64-apple-ios
```

macOS debug builds select the workstation's architecture. The current macOS
release path builds both Apple-silicon and Intel code, so it needs both targets.
Mobile simulator builds select the host simulator architecture; a physical
device build uses the device target instead.

Apple packaging resolves Cargo's target directory through Cargo metadata,
including workspace output locations and `CARGO_TARGET_DIR`. Packaged output
still goes under the host application's `.pax/` directory.

### macOS

For direct native execution without a development server:

```sh
cargo run
```

For template hot reload or an application bundle:

```sh
pax-cli run --target macos
pax-cli build --target macos
pax-cli build --target macos --release
```

`run` builds and opens the development app. A build also prepares the Xcode
project and Swift packages under `.pax/build/<mode>/macos/`, alongside the
`app/` output. A release `.app` is the input to your distribution process;
it is not evidence that signing, notarization, or store submission is complete.

### iOS and iPadOS development

The CLI assembles and launches the mobile app against an available simulator:

```sh
pax-cli run --target ios
pax-cli run --target ipados
```

Full Xcode and an installed iOS simulator runtime are required. No Cargo runner
configuration is needed. The application's native `main` executes in the iOS
process, independently of the workstation CLI.

The default selector chooses an appropriate phone or tablet simulator. List
available simulators with `xcrun simctl list devices available`, then select
one by name or UDID when needed:

```sh
pax-cli run --target ipados --ios-device "simulator:YOUR_SIMULATOR_UDID"
```

Replace the placeholder with an actual identifier. The `--ios-device` flag
applies to both mobile targets. It accepts `simulator`, `device`, prefixed
names/UDIDs such as `device:…`, or an exact device name/UDID. Specify a device
when several are connected.

For a physical-device development run, connect and trust the device, enable
Developer Mode where required, and configure an Apple development team in
Xcode and the project:

```sh
pax-cli run --target ios --ios-device device --ios-development-team YOUR_TEAM_ID
```

Use your real Team ID. This path may ask Xcode to update provisioning or
register the selected device. The flag overrides the target's
`development_team` metadata. Signing errors need to be resolved through your
Xcode account, app identity, and provisioning configuration.

To build a debug simulator app without launching it, use
`pax-cli build --target ios` or `pax-cli build --target ipados`. Device `run`
and simulator `run` are development workflows; they do not produce a TestFlight
or App Store upload.

### Local release runs

Use the same simulator selection with `--release` to build and launch optimized
code without designtime or hot reload:

```sh
pax-cli run --target ios --release
pax-cli run --target ipados --release --ios-device "simulator:YOUR_SIMULATOR_UDID"
```

Run either command for the target you want. Simulator runs do not require
signing credentials. For a paired physical device with Developer Mode enabled:

```sh
pax-cli run --release --target ipados \
  --ios-device 'device:My iPad' --ios-development-team YOUR_TEAM_ID
```

Use `--target ios` for iPhone and replace the device name and team with your
own. The CLI compiles, uses Xcode's Release configuration with Apple Development
signing, installs, and launches the app. The team can also come from
[project metadata](#project-metadata). Release runs disable both hot-reload
lanes even if a flag, environment variable, or project setting requests them.

`pax-cli build --target ios --release` (or `--target ipados`) builds the
device-target app without launching it. These commands do not archive, export,
or upload an app for TestFlight or the App Store.

## Project metadata

Keep application identity and packaging preferences in `Cargo.toml`. Pax reads
`[package.metadata.pax]` while preparing the target interface:

```toml
[package.metadata.pax]
title = "My App"
icon = "assets/app-icon.png"

[package.metadata.pax.web]
title = "My App on the Web"
favicon = "assets/favicon.png"
background_color = "#11110f"

[package.metadata.pax.ios]
bundle_identifier = "com.example.myapp"
development_team = "YOUR_TEAM_ID"
build_number = "1"

[package.metadata.pax.ios.info_plist]
NSCameraUsageDescription = "Take a photo to attach to a note."

[package.metadata.pax.macos]
bundle_identifier = "com.example.myapp.macos"
```

Use identifiers and a team belonging to your project. Include icon/favicon
keys only when those source files exist. Add permission descriptions that
truthfully describe the capabilities your application uses.

Target-specific values override common values. iPadOS first checks
`[package.metadata.pax.ipados]`, then iOS, then common metadata. File paths are
relative to the project root unless absolute; project-relative paths keep the
configuration portable between workstations.

| Key | Applies to | Meaning |
| --- | --- | --- |
| `title` | Common, web, Apple | Browser title or Apple display name; defaults to the Cargo package name |
| `icon` | Common, web, Apple | Image used for generated icons, including the fallback web favicon |
| `favicon` | Web | Explicit favicon file; takes priority over icon-derived favicon generation |
| `background_color` | Web | Opaque `#RGB` or `#RRGGBB` host background, painted from inline HTML before JavaScript/Wasm loads |
| `yield_mount` | Web | Experimental cooperative initial mounting; boolean, defaults to `false`. See [Yield initial web mounting](#yield-initial-web-mounting). |
| `wheel_events` | Web | `"passive"` (default) or `"cancellable"`; controls whether wheel handlers can prevent native scrolling. Applies to the whole mount. See [Wheel input policy](scrolling-viewports.md#wheel-input-policy). |
| `site_name` | Web | Open Graph site name; defaults to the resolved web title |
| `server_owned_prefixes` | Web | Root-relative path prefixes delegated to ordinary browser navigation; defaults to `[]`. See [Server-owned web paths](routing.md#server-owned-web-paths). |
| `site_url` | Web | Absolute public HTTP(S) URL, without query or fragment; required for release builds with indexable concrete routes |
| `social_image` | Web | Default social-preview image: an absolute HTTP(S) URL or a path relative to `site_url` |
| `social_image_alt` | Web | Description of the social image; configure both image fields together |
| `bundle_identifier` | Common, Apple | Application identity for the Apple host |
| `marketing_version` | Common, Apple | User-facing version; defaults to Cargo `package.version` |
| `build_number` | Common, Apple | Apple build number, expressed as a TOML string |
| `development_team` | Common, Apple | Apple development team used during signing |
| `info_plist` | Common, Apple | Typed table of Info.plist entries |
| `info_plist_file` | Common, Apple | Project-owned XML or binary plist with a dictionary root |
| `entitlements` | Apple target only | Typed table of requested signing entitlements |
| `entitlements_file` | Apple target only | Project-owned XML or binary entitlements plist with a dictionary root |

Apple configuration accepts strings, booleans, integers, finite real numbers,
arrays, and dictionaries, including nested dictionaries and arrays of tables.
Reload configuration has its own
[`[package.metadata.pax.dev]` table](developer-workflow.md#configure-a-project-default).

For per-page titles, descriptions, indexing, and social-preview overrides, use
[Web route metadata](routing.md#web-route-metadata). These site-wide settings
supply defaults and public URLs; they do not enumerate parameterized pages.

### Apple property lists and entitlements

Use Apple's exact keys in the target's `info_plist` and `entitlements` tables.
There is no networking-only allowlist: a new Apple key normally needs no Pax
compiler change. Pax preserves value types and writes actual XML property lists
for Xcode to process and sign.

For example, an app that connects to and hosts a TCP service on macOS can opt
into both sandbox permissions:

```toml
[package.metadata.pax.macos.entitlements]
"com.apple.security.network.client" = true
"com.apple.security.network.server" = true
```

Quote dotted entitlement keys so TOML treats each as one key. Client access
permits outgoing TCP connections; server access permits listening for incoming
connections. Once connected, both directions can send and receive. The generated
macOS host retains its default sandbox and user-selected read-only file access;
network access is opt-in. Async Workbench uses both network entitlements for its
loopback TCP exercise. Verify networking in the signed release application:
the unsigned debug host does not exercise the same sandbox restrictions.

An iOS app that discovers nearby services and uses universal links might declare:

```toml
[package.metadata.pax.ios.entitlements]
"com.apple.developer.associated-domains" = ["applinks:example.com"]

[package.metadata.pax.ios.info_plist]
NSLocalNetworkUsageDescription = "Find and connect to nearby devices."
NSBonjourServices = ["_example._tcp"]
```

These are independent examples, not required settings for every async app.
Associated domains also need the corresponding website association file.
macOS client/server sandbox entitlements are not an iOS networking switch.
Local-network privacy applies beneath the networking API, including Tokio;
some multicast/broadcast operations need a separate entitlement. The mobile
debug host adds a local-network usage description for the development connection
when the app has not supplied one; release builds do not add that description.

To keep a larger dictionary in an authored file, or use native plist dates or
binary data, specify a file beside the inline table:

```toml
[package.metadata.pax.ios]
info_plist_file = "apple/Info.extra.plist"
entitlements_file = "apple/App.entitlements"

[package.metadata.pax.ios.info_plist]
NSCameraUsageDescription = "Scan the setup code on your device."
```

Files must have dictionary roots and can use XML or binary plist encoding.
They are overlays on the host's defaults, not complete replacement app manifests.
TOML date/time literals are rejected; use a plist file for dates. Paths resolve
relative to the directory containing the project's `Cargo.toml` unless absolute.
Rebuild and relaunch after changing metadata or either source file.

Precedence is applied in this order, with later values winning:

1. The copied native host's defaults.
2. Common `info_plist_file`, then common `info_plist`.
3. The target's file, then its inline table. For iPadOS, apply the iOS file/table
   first, followed by the iPadOS file/table.

Each override replaces the **whole top-level value**. Arrays are not concatenated
and dictionaries are not recursively merged. An empty array or dictionary can
clear an inherited collection; there is no key-deletion marker. Entitlements
must live under `macos`, `ios`, or `ipados`; there is no common entitlements table
that silently applies platform-specific permissions to every Apple target.

Pax rejects authored entries for managed fields: `CFBundleIdentifier` uses
`bundle_identifier`; `CFBundleName` and `CFBundleDisplayName` use `title`;
`CFBundleShortVersionString` uses `marketing_version`; `CFBundleVersion` uses
`build_number`. The host owns `CFBundleExecutable` and `CFBundlePackageType`.
Application/team identifiers and debugger-access entitlements are owned by
Xcode signing and provisioning. Conflicts are diagnosed in both source files
and inline tables, including entries that another layer would override.

Configuration support does not imply that every capability has native Pax
integration. Usage descriptions still need runtime authorization; restricted
entitlements still need compatible signing/provisioning; background modes still
need the native lifecycle APIs. In particular, enabling Tokio does not keep an
iOS application executing after suspension. Privacy manifests are separate
`PrivacyInfo.xcprivacy` resources and are not generated by these tables.

Use Apple's references for platform availability, types, allowed values, and
additional requirements:

- [Entitlement keys](https://developer.apple.com/documentation/bundleresources/entitlements)
- [Information Property List keys](https://developer.apple.com/documentation/bundleresources/information-property-list)
- [Configuring the macOS App Sandbox](https://developer.apple.com/documentation/xcode/configuring-the-macos-app-sandbox)
- [Local-network privacy](https://developer.apple.com/documentation/technotes/tn3179-understanding-local-network-privacy)
- [Background execution modes](https://developer.apple.com/documentation/xcode/configuring-background-execution-modes)
- [Code signing and provisioning profiles](https://developer.apple.com/documentation/technotes/tn3125-inside-code-signing-provisioning-profiles)
- [Privacy manifest files](https://developer.apple.com/documentation/bundleresources/privacy-manifest-files)

The generated source files are under `.pax/interface/macos/pax-app-macos/pax-app-macos/`
or `.pax/interface/ios/pax-app-ios/pax-app-ios/`: `Info.plist` and
`pax_app_macos.entitlements` or `pax_app_ios.entitlements`. Keep authored inputs
outside `.pax`, which is recreated during builds. An ejected Apple interface
must retain these files and wire its app target's `INFOPLIST_FILE` and
`CODE_SIGN_ENTITLEMENTS` to them, with `GENERATE_INFOPLIST_FILE = NO`.

### Application icons

Use a square source image. iOS and iPadOS require an opaque image and generate
a single 1024×1024 `AppIcon` asset. macOS generates its AppIcon size set from
the square source. Without an explicit web favicon, a configured icon produces
a 64×64 PNG favicon. These are build-time transformations; rebuild after
changing the source or metadata.

The bundled iOS/iPadOS interface includes a Pax icon when no custom icon is
configured. An ejected interface retains its own asset catalog unless an icon
override is supplied.

## Apple distribution

Pax currently builds local macOS/iOS app bundles. Full production packaging and
App Store publication have not been validated end to end on either platform.
Treat the current bundles as inputs to a distribution workflow that still needs
to be completed and qualified, not as a finished store-delivery pipeline.

For macOS, finish the signing and distribution configuration appropriate to
your audience. Direct distribution commonly uses Developer ID signing and
notarization; store distribution follows the App Store process. Apple's
[distribution guide](https://developer.apple.com/documentation/xcode/distributing-your-app-for-beta-testing-and-releases)
and [notarization guide](https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution)
own those platform steps. Pax does not upload an archive or notarize the
application for you.

For iOS and iPadOS, the CLI completes the local release app build and can
development-sign and launch it on a selected device. Distribution still
requires the appropriate Apple archive/export, distribution signing,
provisioning, and upload workflow. A successful local release run does not
establish TestFlight or App Store readiness.

The generated mobile Xcode project is currently at
`.pax/interface/ios/pax-app-ios/pax-app-ios.xcodeproj`; shared Swift packages
and cartridge framework files are under `.pax/interface/common/`. These are
generated working files and can be overwritten by later builds.

A custom native host or manual Xcode integration requires its own release,
resource, signing, and device validation. There is no verified end-to-end
macOS or mobile App Store distribution recipe in this chapter yet. If TestFlight or App Store
delivery is required for your project, plan and verify that distribution step
separately from the CLI's local release workflow.

## Before sharing a build

- Test the release output, including a fresh launch and its primary interaction.
- Check assets and remote dependencies from the deployed location; keep private
  files and credentials out of the build and `public/`.
- Open routed URLs directly, refresh them, and try back/forward navigation.
- Verify the actual target/browser's rendering and native-control behavior.
- Check application title, icon, version, and Apple identity/signing where applicable.
- Keep one complete previous release and a repeatable build procedure.

The shortest shipping loop is a small one: build, serve or install the artifact,
exercise the important behavior, and repeat that check after deployment.

## Read more

- [Getting Started](getting-started.md) — installation and a first successful run.
- [Developer Workflow and Tools](developer-workflow.md) — editing, reload, and inspection.
- [Routing](routing.md) — URL-driven UI and navigation.
- [Text, Fonts, and Images](text-fonts-images.md) — media sources and loading behavior.
- [How Pax Runs](how-pax-runs.md) — runtime, rendering backends, and release behavior.
