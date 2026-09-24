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

## Run, build, and release

Use `run` while developing. Use `build` when you want output without launching
the application:

```sh
pax-cli run --target web
pax-cli build --target web
pax-cli build --target web --release
```

Run these separately. `run` defaults to a debug build and starts the target's
development harness. `build` also defaults to debug; `--release` selects the
optimized release path. iOS and iPadOS additionally accept `run --release` to
install and launch a local release. For web and macOS, use `build --release`.

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

Pax writes generated files beneath the project's `.pax/` directory:

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

Use the project's default Cargo `target/` directory for Apple builds. The
current packaging code looks there for the compiled cartridge; a custom
`CARGO_TARGET_DIR` can let Rust compilation finish and then fail during packaging
because the expected library file is elsewhere.

### macOS

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

Run against an available simulator:

```sh
pax-cli run --target ios
pax-cli run --target ipados
```

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
| `site_name` | Web | Open Graph site name; defaults to the resolved web title |
| `server_owned_prefixes` | Web | Root-relative path prefixes delegated to ordinary browser navigation; defaults to `[]`. See [Server-owned web paths](routing.md#server-owned-web-paths). |
| `site_url` | Web | Absolute public HTTP(S) URL, without query or fragment; required for release builds with indexable concrete routes |
| `social_image` | Web | Default social-preview image: an absolute HTTP(S) URL or a path relative to `site_url` |
| `social_image_alt` | Web | Description of the social image; configure both image fields together |
| `bundle_identifier` | Common, Apple | Application identity for the Apple host |
| `marketing_version` | Common, Apple | User-facing version; defaults to Cargo `package.version` |
| `build_number` | Common, Apple | Apple build number, expressed as a TOML string |
| `development_team` | Common, Apple | Apple development team used during signing |
| `info_plist` | Common, Apple | Nested table of string-valued Info.plist entries |

`info_plist` supports strings here, not arbitrary arrays, dictionaries, or
booleans. Platform-specific entries override matching common keys; iPadOS
also inherits iOS entries. Reload configuration has its own
[`[package.metadata.pax.dev]` table](developer-workflow.md#configure-a-project-default).

For per-page titles, descriptions, indexing, and social-preview overrides, use
[Web route metadata](routing.md#web-route-metadata). These site-wide settings
supply defaults and public URLs; they do not enumerate parameterized pages.

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
mobile distribution recipe in this chapter yet. If TestFlight or App Store
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
