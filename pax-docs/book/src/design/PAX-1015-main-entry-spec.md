# PAX-1015 — User-owned main and initialization

Status: **Ready for implementation review; implementation has not started.**
Investigated 2026-10-04. This document separates observations, feasibility
inferences, and proposed contracts. All new API sketches below are unimplemented.

Issue: [PAX-1015](https://linear.app/paxdev/issue/PAX-1015/design-user-owned-main-entry-points-and-main-driven-initialization).
Related: [PAX-1013](https://linear.app/paxdev/issue/PAX-1013/validate-tokio-support).
Coordinated pre-ship review:
[PAX-1005 — Curate top-level examples, separate fixtures, and scope directory flattening](https://linear.app/paxdev/issue/PAX-1005/curate-top-level-examples-separate-fixtures-and-scope-directory).

## 1. Evidence baseline and approval boundary

The managed investigation checkout is at `0903488598cd37213db27c5569cdf77880587c2e`,
which predates PAX-1013. The clean local main checkout was inspected read-only at
`14b6f8d6a868fc49a2f53668f907a14ab773b640`. Source paths and line numbers in this
document refer to that latter revision, including `29bab61a8` (async) and
`14b6f8d6a` (async tweaks). Neither commit is a proposed wholesale revert target.
No checkout, rebase, commit, implementation, prototype, or API change was made.

Both issue descriptions, all available comments, status, labels, projects and
relations were read. PAX-1015 is In Progress in Pax Core, related to PAX-1013,
with no blocking relations. PAX-1013 is Done in Pax Launch. Its complete merged
`pax-docs/book/src/design/PAX-1013-application-async-spec.md` was read; it explicitly
deferred custom main while delivering independent safety and async work.

Section 9 records the execution baseline and the two-phase scope. No further
product decision is needed to start phase one; platform feasibility and lifetime
correctness remain implementation gates, not assumed results. This readiness
assessment is not a record of an instruction to start implementation. Once
implementation is authorized, remove the superseded public services/custom init
surface before implementing its replacement, preserving the safety kernel in
section 7. Git operations remain Zack's.

## 2. Initial conclusion

A developer-owned Rust process entry is feasible in principle on macOS and
iOS/iPadOS. The present SwiftUI entry is a Pax hosting choice. The platform UI
loop, lifecycle, signing, and main-thread requirements remain real constraints.
Moving the entry does not require replacing the renderer or native controls.

Browser applications can have a user-authored Rust startup function, including
an async one, invoked by JavaScript. That is an application entry, not ownership
of a browser process. Calling it `main` does not create native process semantics.

Process lifetime and cartridge lifetime remain separate. True process startup
runs once. Arbitrary Rust locals, Tokio handles and closures cannot simply cross
the existing C ABI, even when Rust replacement is disabled. Owning a real process
main does not require erasing the cartridge: a thin launcher can invoke an init
facade that resolves it and returns an opaque prepared-app handle.

Selected entry direction (2026-10-04): actual native `fn main` as the standard
standalone entry and explicit `init_web` for the browser, sharing ordinary
typed Rust construction functions **inside the cartridge**. Keep the platform
runners thin. Avoid a second generic service registry. The logic-reload scope constraint is now
resolved as recorded below.

Confirmed by Zack in the subsequent discussion: deliver single-region embedding
on current supported chassis in this task, and use a loading root while async
preparation completes. The execution baseline uses a user-configured Tokio
runtime inside the cartridge with explicit lifecycle adoption, as specified in
section 6; no process-to-cartridge executor ABI is added.

Execution is split into two phases. **Phase one** proves the entry, initialization,
mounting and lifetime refactor while retaining the current `#[main]` root
selection and source-layout convention. **Phase two** introduces explicit
cartridge/root declarations and qualifies nested `ui/` and separate UI-package
layouts after phase one's acceptance checks pass. Keep the internal initializer
and root-adoption seam independent of the marker so phase two can change
discovery without redesigning startup or lifetime ownership.

Further direction from Zack: preserve the cartridge as a real module/linkage
boundary, with room for future dynamic linking. Keep Android's future shared
library and web's Wasm module boundary; do not flatten the existing Apple
cartridge simply to obtain a main. A Wasm module loaded by JS is not a promise
of general-purpose Wasm dynamic linking. Removing live replacement is independent
of retaining separately linked artifacts.

### Discussion decision: Rust logic reload is not a requirement

On 2026-10-04 Zack said backing out, feature-flagging or stubbing Rust hot reload
is acceptable if that simplifies this work. This resolves the requirement to
preserve in-process Rust replacement; it does not approve implementation or select
a particular code-removal strategy yet.

Revised recommendation: rebuild/restart for Rust changes on native targets and
rebuild/reload the page on web, retaining `.pax` hot reload. Do not design a new
process-to-cartridge resource ABI for this task. Prefer disabling the Rust logic
lane over offering a stub that appears to work or adding a new supported feature
flag. Keep old internals only if they remain isolated and do not constrain the
new ownership model; otherwise remove them selectively after spec approval.
Retain normal cartridge loading, exports and ownership. This recommendation
removes replacement, not dynamic linkage itself.

The current default is already `pax`, not `all`: verified in
`pax-compiler/src/hot_reload.rs:8` and
`pax-docs/book/src/developer-workflow.md:17` at the evidence revision. CLI,
environment and project metadata can opt in to logic reload. The present default
requires a manual rebuild/restart for Rust changes; automatic restart-on-save is
not implicitly added to this task.

## 3. Current startup and ownership

### Artifact map

| Layer | Current artifact and owner | Evidence |
| --- | --- | --- |
| Application source | Library target, commonly `cdylib` plus `rlib`; annotated main component is a root designation, not Rust process main | `examples/src/async-workbench/Cargo.toml:28`; `src/lib.rs:8` |
| Generated cartridge | Rust source injected into active root crate; root dependencies suppress platform entry exports | `pax-macro/templates/derive_pax.stpl:1`; `pax-macro/src/templating.rs:103` |
| Browser | `wasm-pack build --target web`; JS glue plus `pax-cartridge_bg.wasm`; TypeScript host controls mount and frames | `pax-compiler/src/building/web.rs:341`; `files/interfaces/web/src/index.ts:142` |
| macOS | Rust cdylib embedded in a framework/app; SwiftUI executable is the process owner; symbol table allows debug replacement | `pax-compiler/src/building/apple.rs:473`, `:519`, `:606`, `:1988`; macOS `PaxViewMacos.swift:241` |
| iOS/iPadOS | Rust cdylib packaged into framework/xcframework slices; SwiftUI app executable calls C exports | Apple builder `:644`; iOS `Main.swift:10`; `PaxViewIos.swift:502` |
| Example CLI launcher | A separate executable that launches `./pax run`; its runtime is not the GUI application's runtime | `examples/src/runtime-settings-themes/bin/run.rs:4` |

Apple paths in this section are under
`pax-compiler/files/interfaces/{macos,ios}/pax-app-{macos,ios}/pax-app-{macos,ios}`.

### Browser callgraph

```text
page -> TypeScript mount(element, artifact URL)
  -> startRenderLoop -> loadWasmModule
  -> import JS glue, fetch Wasm, await glue default initialization
  -> install independent shutdown waker
  -> await generated pax_init()
       -> ApplicationInstance::prepare::<SelectedApplication>(target)
            -> create graph, enter graph, Application::configure(AppBuilder)
            -> freeze services, create staged dispatcher and app scope
       -> enter application/graph; initialize fallback logger
       -> init_manifest + generated traverser
       -> PaxChassisWeb::new[_designtime]
            -> main component descriptor -> PaxEngine::new -> root initialization
  -> attachChassis: install property waker -> activate_application
       -> started hook -> release staged task starts
  -> native pool, route/viewport/input integration -> scheduled frame loop
```

References: web `index.ts:181`, `:272`; macro template `:10`;
`pax-chassis-web/src/lib.rs:320`, `:360`, `:479`;
`pax-runtime-api/src/application.rs:249`, `:320`;
`pax-runtime/src/engine/mod.rs:440`, `:483`.

The exported TypeScript `mount` currently returns no readiness Promise. Internal
startup awaits `pax_init`, but its top-level catch logs an error. A future
`init_web` contract should propagate startup failure to its caller explicitly.
Activation also currently precedes native-pool attachment and browser route/
viewport setup; the new readiness contract must deliberately specify that order.

### macOS callgraph

```text
SwiftUI @main pax_app_macosApp -> WindowGroup -> PaxViewMacos
  -> PaxCanvasViewMacos -> initial tick / frame scheduling
  -> PaxCartridgeRuntime loads cartridge and resolves C entry table
  -> install shutdown wake route -> paxInit(width, height)
       -> generated C pax_init -> prepare/configure -> graph/application enter
       -> manifest + traverser + main component -> PaxEngine::new -> root
  -> install generation-checked property waker -> activate application
  -> CoreVideo signals enqueue main-thread tick/render/native work
```

References: macOS `Main.swift:88`; `PaxViewMacos.swift:241`, `:402`, `:932`,
`:1086`, `:1103`, `:1209`; macro template `:41`.
The host renders an initial frame without relying on successful display-clock
startup and retries clock failures. Preserve that PAX-1013 fix.

### iOS/iPadOS callgraph

```text
SwiftUI @main pax_app_iosApp -> WindowGroup -> PaxViewIos
  -> PaxCanvasViewIos -> CADisplayLink on main run loop / tick
  -> install shutdown waker -> linked pax_init(width, height)
       -> same native prepare/configure and engine/root construction
  -> install property waker -> pax_activate_application
  -> UIKit/display-link callbacks -> serialized Pax tick/render/interrupts
```

References: iOS `Main.swift:10`; `PaxViewIos.swift:155`, `:422`, `:498`.
iPadOS shares this host with target-specific build metadata. Logic changes
currently require rebuild/relaunch on mobile.

### Ownership that must survive changing the entry

`ApplicationInstance` currently owns the graph, dispatcher, app scope, typed
services, hooks and shutdown state. `RuntimeContext` attaches it to the engine.
`NodeContext::application()` exposes a weak local context. `PaxEngine::shutdown`
closes authority before unmount and resource cleanup (`engine/mod.rs:317`).
`application_shutdown.rs` retains closing owners and wakes cleanup independently
of a disposed view. The Tokio adapter owns runtime destruction on a separate
thread; borrowed mode leaves the runtime with its original owner.

Application identity is useful without application services. Do not delete this
entire lifetime layer merely because its current file also contains a registry.

Current host limits matter for embedding: web module globals reject duplicate
mount, and Apple hosts use shared engine/native-element containers. The async
design's intended per-app isolation does not prove simultaneous multi-window or
multi-mount host support. Do not silently promise that as part of main support.

## 4. Platform feasibility and constraints

| Target | True Rust process main | Application entry | Event-loop / lifetime constraint | Assessment |
| --- | --- | --- | --- | --- |
| macOS | Rust executable can own main and call an Apple host bridge | A function called by existing SwiftUI host is also possible | AppKit/SwiftUI launch and all UI work stay on main; ordinary app termination need not return through Rust stack | Feasible inference; new packaging/link proof required |
| iOS / iPadOS | Rust supports these executable targets; launch can reach SwiftUI `App.main()` through a bridge, or `UIApplicationMain` through UIKit integration | Existing host can call an exported Rust bootstrap | UIKit owns event cycle; `UIApplicationMain` does not return; backgrounding is not exit | Feasible inference, not a completed Pax/iOS proof |
| Browser | No ownership of browser process | Explicit exported sync/async function or synthetic main | JS event loop retains control; asynchronous work must yield; no dependable tab-close cleanup | Directly compatible with current exported-startup pattern |
| Android (future; no current Pax build target) | A conventional executable main is not the normal Android application entry | Activity/JNI or native-activity callback into a shared library; a future Pax `init_android` would adapt this | Activity lifetime and its UI thread remain platform-owned; a native-glue thread is distinct | Architectural constraint only; no Android chassis work included |

Authoritative evidence:

- [Rust iOS target documentation](https://doc.rust-lang.org/rustc/platform-support/apple-ios.html)
  covers compiling programs and SDK requirements. This supports feasibility,
  not proof of a signed Pax package or App Store acceptance.
- [SwiftUI App.main](https://developer.apple.com/documentation/swiftui/app/main())
  is a public main-actor launch method. `@main` selects it as the generated entry.
  Inference: a Rust entry can call a thin bridge that invokes this method while
  preserving Pax's existing SwiftUI scenes. Exact Swift export/link integration
  remains a qualification gate.
- [UIApplicationMain](https://developer.apple.com/documentation/uikit/uiapplicationmain(_:_:_:_:)-1yub7/)
  creates the application/delegate and event cycle and does not return normally.
  Direct UIKit ownership would require rebuilding delegate/scene hosting around
  current Pax views; it is a larger alternative than retaining SwiftUI.
- [NSApplicationMain](https://developer.apple.com/documentation/appkit/nsapplicationmain)
  requires the main thread and may terminate via exit. AppKit's lower-level
  [run](https://developer.apple.com/documentation/appkit/nsapplication/run()) can
  stop, but a returning runner would need a deliberate termination policy.
- [Apple launch guidance](https://developer.apple.com/documentation/uikit/responding-to-the-launch-of-your-app)
  requires prompt launch and moving long tasks off the main thread. Owning main
  does not make blocking startup I/O appropriate.
- [Rust main rules](https://doc.rust-lang.org/reference/crates-and-source-files.html#main-functions):
  a normal main has no arguments and returns a `Termination` type. A library
  function named main is not automatically an executable entry.
- [wasm-bindgen main](https://wasm-bindgen.github.io/wasm-bindgen/reference/attributes/on-rust-exports/main.html)
  supports async Cargo binary entry, but does not turn a library function into
  an automatic start. [start](https://wasm-bindgen.github.io/wasm-bindgen/reference/attributes/on-rust-exports/start.html)
  runs at module initialization; that is too early for arbitrary host mount
  arguments and prepared-candidate control. Prefer an explicit export here.
- [JS Promises and Rust futures](https://wasm-bindgen.github.io/wasm-bindgen/reference/js-promises-and-rust-futures.html)
  provides a natural fallible async exported entry. An export receiving a mount
  element can return a Promise for a mounted handle rather than hidden globals.

Read-only SDK confirmation: installed iPhoneOS 26.4 `UIApplication.h:546` declares
`UIApplicationMain`; the arm64e SwiftUI interface declares public main-actor
`App.main()` at lines 8157–8164. No bridge code was compiled or written.

### Entry naming, mobile targets and authoring ergonomics

Zack selected `fn main` and the name `init_web` on 2026-10-04. Native does not mean
desktop-only: the proposed standalone Apple build owns a Rust process main on
macOS, iOS and iPadOS. On mobile it promptly hands over to the SwiftUI/UIKit
launch path, which remains responsible for the UI loop. This remains subject
to the Apple packaging/linking feasibility gate. No separate `init_ios` is
needed for this standalone design. An embedded view in someone else's iOS app
uses a mount API, because that app already owns its process entry.

Use a clear naming rule: `main` for a user-owned executable entry; `init_<host>`
for a host-invoked application entry. The latter receives host facts or handles,
returns readiness/failure and an owned session, and does not claim process
lifetime. Calling a function `main` from an Android callback would only make it
a synthetic main and should not be documented as restoring process ownership.

Android's [NDK application model](https://developer.android.com/ndk/guides/concepts)
loads native code as a shared library and invokes activity callbacks.
[GameActivity's glue](https://developer.android.com/games/agdk/game-activity/get-started)
can call `android_main` on a separate native thread; that name does not imply the
Android UI thread or a conventional executable entry. A future `init_android`
could hide platform ABI details behind a typed host context. Its threading and
native-control integration must be designed with that chassis, not guessed now.
Activity recreation also differs from process restart; see the
[Activity lifecycle](https://developer.android.com/guide/components/activities/activity-lifecycle).
Do not repeat process-global initialization unconditionally on each recreation.

Repository support is narrower than the platform enum: `RunTarget` currently
contains macOS, Web, iOS and iPadOS (`pax-compiler/src/lib.rs:150`). Android appears
in platform facts but has no corresponding CLI target. Windows and Linux are
currently web-build workstations, not native Pax destinations
(`pax-docs/book/src/targets-build-deploy.md:19`). Future Windows/Linux chassis
naturally fit the process-main model; this task adds neither those chassis nor
Android support.

Phase-one starter organization preserves the existing component location and
root marker while adding a real launcher and ordinary initializer:

```text
src/main.rs        process entry and host policy
src/lib.rs         existing #[main] root component and library exports
src/lib.pax        existing root template
src/app.rs         ordinary initialization and resource ownership
```

Phase-two proposed default source organization, not a discovery rule:

```text
src/main.rs        process entry and host policy
src/lib.rs         module wiring and cartridge declaration/exports
src/ui/mod.rs      UI module and public initializer/component exports
src/ui/app.rs      ordinary Pax root component and handlers
src/ui/app.pax     its template
src/ui/startup.rs  user-authored initialization and resource ownership
src/domain/       non-UI application logic, when needed
```

Native main calls a host-facing init facade. The default Apple package retains
its separate cartridge; web builds the library export. A direct sibling-library
Rust call is an optional statically linked deployment, not equivalent to loading
the cdylib. Build metadata must explicitly select the appropriate binary/library
and generated descriptor owner. Do not encode the rule as
`not(wasm32) => executable main`, which would be wrong for Android hosting.
Generated projects should supply small editable entry functions. They should
not require users to implement a new application lifecycle trait merely to share
startup code. One ordinary shared root factory can serve both entries; runtime,
filesystem and host arguments stay in their appropriate platform wrappers.

API/docs should teach two separate operations: launch a standalone host, and
mount a session into a host that already exists. Both use the same typed root
construction, scoped-work, readiness and close concepts. A native runner retains
the platform loop until exit; web initialization returns an owned mount handle
after readiness, and must not wait for the entire app lifetime. `init_web` is
explicitly invoked by the page after host/module setup, never by merely importing
a component library. Initially reject duplicate live mounts consistently with
current host limits. Document call count, owning thread, available host arguments,
readiness, failure and disposal together for each entry.

### Root selection and coexistence with an existing codebase

**Phase two, after proving phase one.** This section records the intended
follow-up contract, not a prerequisite for the entry/lifetime refactor. Phase
one retains `#[main]`, current package selection and template path resolution;
it changes generated startup plumbing only as required for user-owned entry and
typed initialization. Do not ship `pax::cartridge!`, migrate component locations,
or claim separate UI-package qualification as part of phase-one completion.

Zack raised the need to make Pax a flexible neighbor in a codebase, including
putting UI under a `ui` directory. Recommendation: separate the executable entry,
the cartridge declaration and the root component's source location. None of
`main.rs`, `lib.rs`, `App`, `Main` or a directory name determines the UI root.
The layout above is a starter template, not an imposed application architecture.

Current source is already more nuanced than a main.rs convention:
`examples/src/increment/src/lib.rs:6` uses `#[main]` on a component; macro
configuration recognizes that attribute, and the active application's marked
component injects cartridge source and platform exports. Package identity is
checked through PAX_DIR/CARGO_MANIFEST_DIR rather than the component filename
(`pax-macro/src/root_crate.rs`). This does not prove every arbitrary nested-root
layout works end to end today. The change needs to decouple entry/code-generation
ownership from the component marker, not simply rename a file.

Preferred declaration shape, illustrative and unimplemented:

```rust,ignore
// src/lib.rs: library wiring; UI implementation remains under ui/.
pub mod ui;

pax::cartridge! {
    root: crate::ui::App,
    init: crate::ui::init,
}
```

`ui::App` is an ordinary `#[pax]` component; `ui::init` is an ordinary typed
initializer returning that root. Check their types for agreement. The declaration
provides explicit build-time root discovery and generates the native/web adapter
plumbing once for the selected cartridge. Do not discover a root by executing
main/init, scanning arbitrary runtime mount calls, guessing filenames, or requiring
the component itself to define process entry behavior. Exact macro spelling can
be settled with the rest of the API; the separation is the proposed contract.

Root is a role in a selected program, not an intrinsic restriction on a component
type. The same component can be a subtree in another application. Ordinary Rust
module visibility applies; a component crossing a crate boundary must be exposed
through that crate's public API. A separate workspace `ui` package can own the
cartridge and initialization while the host package owns main. Build configuration
must explicitly identify the cartridge package/library target separately from
the executable target, and must not assume the primary executable package owns
the generated cartridge. This extends the single-package starter layout without
adding multiple simultaneous mounts or arbitrary dynamic root selection.

Keep one selected root/initializer per cartridge build initially. Old `#[main]`
can be a documented compatibility shorthand during migration when no explicit
declaration is supplied; conflicting declarations must produce an actionable
error, not generate two startup paths. Migrate repository examples to the
explicit composition model. The component attribute should no longer be the
sole owner of cartridge injection and platform exports.

Co-locating files must not imply a path behavior we have not implemented.
Current `#[file(...)]` resolution is package-relative with a `src/` fallback,
not relative to the containing Rust module. For the phase-two layout above, the
existing unambiguous spelling is `#[file("src/ui/app.pax")]`. Keep that resolution
in both phases; changing the path base would be a separate syntax/migration decision.
See `components-composition.md:129` and `pax-macro/src/lib.rs:678`.

Qualification must include a nested UI module and a host consuming a distinct
workspace UI cartridge package. Exercise source discovery, template/asset paths,
watching, dependency components, dev inspection and debug/release generation.
Derive the selected root's manifest identity from its qualified type path and
audit existing `main_component_type_id`/`is_main_component` production through
Rust-manifest and binary/IR paths. Source-layout flexibility must not become a
debug-only capability. No public docs should claim this new contract is already
implemented; update Getting Started and Components & Composition with migration
examples when implementation lands.

### Recommended Cargo application template

One package can contain a library and an executable. Keep app code in the library;
give the launcher its own binary target. Existing app templates already use
`crate-type = ["cdylib", "rlib"]`; their `bin/run.rs` is currently a CLI wrapper,
not the application's process entry. Replace that ambiguity deliberately.

Proposed manifest skeleton; ordinary dependencies and Pax metadata omitted:

```toml
[package]
name = "my-app"
version = "0.1.0"
edition = "2021"
default-run = "my-app"

[lib]
path = "src/lib.rs"
crate-type = ["rlib", "cdylib"]

[[bin]]
name = "my-app"
path = "src/main.rs"
required-features = ["standalone"]

[features]
default = []
standalone = []
```

`standalone` selects the launcher; it is not a synonym for `not(wasm32)`, nor
does it select a platform. Pax's native standalone build enables it and selects
the binary explicitly. Hosted/library builds select `--lib` without enabling
it. Direct launcher selection uses `--bin my-app --features standalone`; that
does not by itself replace Pax code generation, assets, signing or packaging.
Keeping it opt-in avoids accidentally building a meaningless executable when
cross-compiling a library. `default-run` only selects a binary; it does not
enable its required features. No dummy Wasm/Android main is needed.

The [Cargo target reference](https://doc.rust-lang.org/cargo/reference/cargo-targets.html)
supports separate lib/bin targets, multiple library output types and
`required-features` gating for binaries. The gate does not apply to the library.
Target-specific dependencies and cfgs still keep Apple hosting dependencies out
of browser/Android compilation. A reusable component-only crate usually needs
only an ordinary Rust library, not this full application template.

| Deployment | Artifact selected | Entry and linkage |
| --- | --- | --- |
| Standalone macOS, proposed iOS/iPadOS | Launcher binary plus packaged cartridge | Rust main invokes thin Apple host bridge; retain current cartridge cdylib and its exports |
| Browser | Library built as Wasm plus JS glue | Page calls explicit `init_web`; no native binary |
| Future Android | Library built as `.so` | Activity/JNI or native glue invokes app initialization; no ordinary Rust executable |
| Embedded Apple app | Packaged library and platform view adapter | Existing app owns entry and UI loop |
| Optional single-image Rust deployment | `rlib` linked into native executable | Same opaque host/session contract; enabling this mode is not required for this task |

Both library artifacts do not imply that Cargo connects the binary to the
cdylib. A normal `my_app::...` Rust dependency call uses Rust linkage, generally
the rlib. Preserving a separate native cartridge requires explicit generated
link/load integration. The launcher must not also instantiate the app through
its sibling rlib. Keep the host SDK separate from cartridge-owned engine state;
do not rely on shared Rust globals or thread-local identity across artifacts.

Use `cdylib` for the system library boundary; `rlib` supports Rust reuse and a
possible linked deployment. Do not replace cdylib with Rust `dylib` as a shortcut.
Emit `staticlib` only for a deliberately supported foreign-host static packaging
mode, not by default for every target. See the
[Rust linkage reference](https://doc.rust-lang.org/reference/linkage.html).

### Cartridge contract retained beneath init

The host-facing `pax::init(options)` sketch resolves the build-selected bundled
cartridge and returns an opaque, one-use `PreparedApp`. It does not enter a UI
loop or construct UI-affine root state. `run` or `mount` attaches a host, enters
the cartridge's graph, and invokes the selected application initializer there.
Library exports and the linked implementation share this lifecycle contract.
Generated platform adapters hide artifact discovery and FFI; ordinary builders
should not write a symbol loader. Exact API spelling is still proposed.

Keep typed root values, Property allocations, captured closures, Rust task
handles and their destructors inside their defining cartridge. The native ABI
uses opaque instance IDs/pointers, explicitly laid-out values/byte buffers and
defined callbacks, with release functions on the allocating side. Version/size
or build compatibility checks reject mismatches before invocation. Retain the
library until instances, callbacks and cleanup have finished. This is an
evolution of the existing boundary, not a new public plugin ecosystem or a
promise of indefinite binary compatibility. Web has equivalent ownership through
its JS/Wasm bindings, not the identical native C representation.

User-defined startup data may have a generated encoding at a foreign boundary;
arbitrary Rust objects cannot be treated as configuration bytes. Typed convenience
APIs within one Rust linkage unit remain possible. Distinguish them from the
portable host contract. FFI layout, callback and unwind constraints follow the
[Rust FFI guidance](https://doc.rust-lang.org/nomicon/ffi.html).

## 5. Credible alternatives and authoring sketches

These are API design illustrations, not buildable examples or new API promises.
`Root` is a Pax component; construction functions live inside the cartridge.

### A. Native process main plus explicit browser entry — selected direction

```rust,ignore
// src/main.rs, native executable target
fn main() -> Result<(), LaunchError> {
    let app = pax::init(InitOptions::default())?;
    pax::native::run(app, WindowOptions::default())
}

// Library export, selected explicitly by the generated web host.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub async fn init_web(target: web_sys::Element) -> Result<WebMount, JsValue> {
    let app = prepare_app()?;
    pax::web::mount(target, app).await
}
```

`prepare_app` configures the cartridge's initializer without constructing the
root outside its UI graph. Web may first await application configuration using
browser I/O, or mount a Loading root. The underlying typed root initializer is
shared with the native cartridge export; it is not a closure passed from the
native launcher across C. `LaunchError`, `PreparedApp`, `WebMount`, `init`,
`native::run` and `web::mount` are proposed names. The web return type requires
a generated wasm-bindgen-compatible wrapper.
Native run may report errors before platform handoff, but does not promise to
return after normal iOS or SwiftUI process termination. Later startup failures
need the host error channel, not only the synchronous Result.

Factories run once per requested application instance on the UI thread, with
the owning property graph entered. This lets root defaults create local graph
state correctly. Ordinary shared input data may be created before the factory.
The supplied root must become the actual root; silently constructing a second
default root would defeat the initialization feature.

Benefits: honest native control; retained cartridge boundary; visible web
semantics; shared construction is ordinary Rust inside the cartridge. Costs:
Apple launcher/build restructuring and explicit initializer/root discovery.
Process-to-cartridge Rust resource injection needs its own contract; removing
replacement does not remove that constraint.

### B. One annotated main with generated target adapters

```rust,ignore
#[pax::entry]
fn main() -> Result<Root, StartupError> {
    Ok(build_root(load_config()?))
}
```

An adapter can lower this to a real native entry and an explicit web export, but
the written signature is not a conventional Rust main. It cannot pretend that
returning a root means the same lifetime as a blocking native run. An async
version additionally needs a named executor policy per target. This is compact
for beginner applications but introduces macro semantics around setup, ownership,
error conversion and return. Do not make it the only control surface.

### C. Shared synthetic application entry under existing hosts

```rust,ignore
// Ordinary library function selected through explicit build metadata.
pub fn app_main() -> Result<Root, StartupError> {
    Ok(build_root(load_config()?))
}
```

Generated `pax_init` calls it after installing the lifetime/graph boundary. No
SwiftUI entry change is necessary, and prepared-candidate reload fits naturally.
This offers custom construction with lower platform risk. It does not restore
actual process main; locals die when it returns unless ownership moves into the
root/session. Adding a Tokio entry macro to it and returning immediately would
shut down that runtime. It is a credible alternative if uniform reload matters
more than conventional executable ownership.

### D. Direct UIKit/AppKit Rust runner

Native main calls UIKit/AppKit launch functions through bindings, retaining Swift
view bridges where needed. This gives deeper delegate/window control but requires
more scene, lifecycle, menu, activation and packaging work. Merely removing
SwiftUI `@main` does not implement those responsibilities. Prefer the thin SwiftUI
launch bridge first unless Zack wants host ownership itself redesigned.

## 6. Proposed semantic contract, independent of spelling

### Initialization and root state

1. Process main performs bounded process setup and resolves a cartridge through
   the init facade. Host creation establishes main-thread validation, launch
   facts, error reporting and shutdown scheduling. A prepared handle has not
   yet constructed a root; dropping it must release its retained resources.
2. An application instance gets fresh identity, property graph, delivery limits
   and cancellation scope. None of these require a service type map.
3. Under that graph, invoke the supplied fallible root factory exactly once.
   Keep root values and dependency handles as ordinary typed Rust ownership
   inside the cartridge, with only an opaque session exposed to the host.
   Existing lexical stores can share dependencies down a subtree; do not turn
   them into a renamed global service registry.
4. Resolve program descriptors, adopt the supplied root state, apply documented
   template/settings precedence, build bindings, and initialize/mount the tree.
   Proposed precedence: explicit root construction replaces Rust defaults;
   template/settings bindings still apply under existing precedence rules.
   Verify this with existing initialization tests before treating it as settled.
5. Attach host surfaces, native element pool, current viewport/routes and wake
   handlers before public ready/activation notification; gate scoped work until
   commit. Ready means mounted and accepting work, not all fonts/assets loaded.
6. Start normal input/frame processing. Component mount/unmount remains separate
   from process startup. Dependency components never start a second app.

Typed root adoption must preserve property identities, including shared inputs
and local values. It must not round-trip the supplied root through manifest
serialization. Today `get_main_component` selects descriptor defaults
(`pax-runtime/src/cartridge.rs:1332`, `:1380`); `PropertiesInit::Factory` offers
an internal integration point (`pax-runtime/src/rendering.rs:123`). This is an
implementation seam, not evidence that public root injection already exists.

### Executors and asynchronous startup

The engine remains synchronous and UI-owned. Tokio remains optional. With a
separately linked cartridge, the portable default lets **user-authored application
code construct and configure its own executor inside the cartridge**. Neither the
host-facing `pax::init` nor the engine silently creates a Tokio instance. A
synchronous application needs none. Scope, error, cancellation and guarded
publication remain without service lookup; owned executors stay retained through
app cleanup.
Main still owns process policy, launch configuration and host choice. A process
logger or executor singleton must not be assumed to configure a separately
linked copy automatically.

Proposed native initializer example, selected by generated cartridge entry
metadata and invoked once on the UI owner with its graph entered. Only Tokio's
Builder calls below are existing API; Pax integration names are provisional:

```rust,ignore
// src/app.rs in phase one, compiled into the application/cartridge library
fn init_app(cx: &pax::InitContext) -> Result<Root, AppError> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .thread_name("my-app-worker")
        .enable_all()
        .build()?;

    let executor = pax_tokio::Executor::owned(runtime, cx.lifetime())?;
    Ok(Root::new(executor))
}
```

`init_app` is an ordinary user-authored initializer, not an Application trait
implementation or registration callback for a hidden executor. Root stores the
explicit executor client as ordinary typed state. The lifetime argument denotes
a proposed narrow integration with app scope and retained shutdown, not a type
map: owned construction transfers the exact supplied runtime to its off-UI owner;
session close initiates shutdown and retains cleanup and cartridge code until
acknowledgement. Root/client drop alone must not bypass that retention.
No second runtime is constructed and the adapter does not reconfigure the
provided runtime. This makes user control of Tokio Builder and Pax's lifecycle
coordination separate responsibilities.

The application's normal Tokio-based libraries and futures use that runtime.
Pax-bound task helpers additionally provide staged start, node/app cancellation,
failure reporting and guarded UI publication. Raw Handle spawning remains
possible but is untracked by Pax scopes. UI callbacks are not implicitly inside
a Tokio runtime context; explicit handles/helpers should be the teaching path.
The [Tokio Builder](https://docs.rs/tokio/1.53.1/tokio/runtime/struct.Builder.html)
exposes runtime configuration directly. This recommendation preserves that
control rather than translating it into a Pax-specific set of Tokio settings.

Current evidence: `examples/src/async-workbench/src/work_service.rs:12` already
constructs its own Runtime through Builder, and `pax-tokio/src/lib.rs:64` consumes
that exact runtime in `TokioService::owned`. The change removes public
`provide_managed`/service-lookup coupling while preserving off-UI destruction,
tracking and cleanup acknowledgement. A same-linkage borrowed Handle remains
an option when the application already owns a continuously driven runtime;
its owner remains responsible for keeping it alive through cleanup. Do not
silently create a replacement runtime when a supplied handle is invalid.

Direct main-owned Tokio Handle injection is possible in an explicitly linked
Rust deployment. It is not part of the native C or browser contract. Illustrative
linked-only convenience, not the default separately packaged Apple path:

```rust,ignore
fn main() -> Result<(), LaunchError> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all().build()?;
    let app = my_app::prepare_with_executor(runtime.handle().clone())?;
    pax::native::run(app, WindowOptions::default())
    // No portable guarantee that platform termination returns here.
}
```

This optional linked mode does not replace the default cartridge-owned executor.
A borrowed Handle does not retain the runtime owner. Requiring the same
main-owned executor through a separately loaded cartridge would add a host task
capability/FFI design; casting a Handle or a Rust future is not that design.
Execution baseline: do not add that bridge to this task. Keep the distinction
visible in docs; same-linkage borrowing does not imply cross-artifact injection.

Use manual runtime construction wherever the runtime is owned. `#[tokio::main]` is not
inherently moved to a worker: its outer future runs on the calling thread. But
putting a blocking Apple event loop inside it is not a general UI executor
integration. Current-thread Tokio and LocalSet futures would stop progressing
when their driver is occupied by the platform loop. Keep the proven dedicated
service-thread pattern; do not add tick polling or claim UI-affine Tokio support.
See [Tokio main](https://docs.rs/tokio/1.53.1/tokio/attr.main.html) and
[Runtime](https://docs.rs/tokio/1.53.1/tokio/runtime/struct.Runtime.html).

Selected native async startup: enter the platform promptly, mount a loading root,
and publish asynchronous preparation results. Browser `init_web` may await
configuration before mounting while yielding to JS; report cancellation/error
and bound waiting at the application level. If a portable await-before-root API
is required, explicitly design a main-queue future executor/loading host as added
scope. Do not fake it with `block_on` or poll futures from display ticks.

#### Loading state does not mean waiting on the host thread

The selected approach means the application's real root starts with a Loading state
and later changes to Ready or Error. It need not construct a throwaway root and
replace it. Its quick synchronous construction/mount happens on the graph's UI
owner, then normal platform event processing continues while initialization is
pending. `native::run` may not return to its caller; that means it is running the
UI event loop, not waiting synchronously for data initialization.

```text
UI owner:  short setup -> mount Loading -> input / frames / other events -> Ready
workers:                    start preparation --------> publish result --^
```

Native initialization tasks run on the selected continuously driven executor.
Tokio multithread tasks run on workers; blocking or CPU-heavy jobs require bounded
blocking/compute workers, not long synchronous sections inside UI callbacks.
Results publish shared state or use existing bounded delivery; local graph
settlement and view mutations occur only on the UI owner. Do not join workers,
wait for channels, or call `block_on` from the host's UI callbacks.

Browser initialization uses browser I/O promises/futures and yields to JS.
The continuation still runs on the browser thread; `spawn_local` is not a Web
Worker ([browser-local future execution](https://wasm-bindgen.github.io/wasm-bindgen/api/wasm_bindgen_futures/fn.spawn_local.html)).
`async fn` alone never moves work to another thread. Heavy synchronous code before
or between awaits can still freeze the page. Worker execution is separate target
capability work and is not implicitly promised by this startup API.

Pax's guarantee should be that its startup machinery never synchronously waits
for application initialization on the UI owner. It cannot preempt arbitrary
user code in a root factory, future poll, or event handler. Document short
construction and nonblocking I/O separately from background computation; prove
responsiveness with a held initialization request and independent UI interaction.
Zack agreed to the loading-root approach in the subsequent discussion. A portable
native await-before-root executor is outside this task; browser init may still
await nonblocking browser I/O as described above.

Browser adapters use browser timers/fetch and local future execution, with the
same Pax lifetime guards. No claim of desktop Tokio sockets/threads on browser
Wasm: [Tokio Wasm support](https://docs.rs/tokio/1.53.1/tokio/#wasm-support) and
[Rust browser target limits](https://doc.rust-lang.org/rustc/platform-support/wasm32-unknown-unknown.html).

### Errors, close and resource ownership

- Separate pre-handoff launch error, asynchronous preparation error, mounted
  application fatal error, and background task failure. Web callers receive a
  rejected startup Promise; native post-handoff failures need a visible host
  diagnostic. Never unwind Rust through C; aborting panics are not recoverable.
- Closing invalidates app/node/operation authority before unmount. Stop new
  scoped work, revoke guarded publication, cancel tracked work, and drain cleanup
  through independent host wakeups. Preserve the current bounded queues and
  UI-only callback/drop rules.
- Any borrowed runtime must outlive app instances; closing one app must not
  destroy it. Cartridge-owned executors are the recommended default; retain
  their nonblocking destruction and acknowledgement machinery. Removing
  public registration is not permission to drop Tokio on the UI thread.
- Cleanup ownership: a cartridge-local session owner retains the root,
  lifecycle adapters and cleanup tickets. The host mount retains the cartridge
  lease and session connection. On close/drop, transfer unfinished cleanup to
  a retained teardown owner driven by host wakeups, so dropping the public
  handle cannot unload code or destroy a runtime before acknowledgement.
  `closed()` observes completion; polling that observer is not what drives
  cleanup. Keep current tickets internally until replacement retention and
  ordering are proven. Ordinary typed resources and a close notification do
  not introduce generic `provide`/lookup. Process exit remains a separate
  boundary from session disposal.
- Mobile backgrounding is suspension, not destruction. Abrupt kill, forced quit
  and tab close cannot guarantee cleanup. Persistence must not depend on main
  returning or destructors executing. Normal embedded disposal is explicit and
  awaitable where the host can continue running.

### Reload and embedding

Pax-only reload preserves process setup and retained root/application state;
actual node unmount still invalidates node scopes. Entry, process configuration
or runtime changes require restart. Release disables both reload lanes.

In-process Rust replacement is outside the revised supported scope on native
and web. Native applications rebuild/restart; browser applications rebuild and
reload the page. Neither path transfers live Rust resources to a new cartridge.
This removes the need for a reload factory/host-service ABI from this task.

Proposed CLI migration follows the existing mobile precedent: `pax` and `off`
retain their meanings; `all` keeps `.pax` reload and visibly reports that Rust
changes need rebuild/restart; `logic` alone is rejected with that guidance.
Apply the same policy to environment and Cargo metadata, update help/docs and
test diagnostics. Do not silently accept unsupported logic replacement.

Normal failed-startup, application disposal and node-unmount cleanup remain
required. Retain staged work until initial activation, independent shutdown
wakes and outstanding-work retention. If legacy replacement code is retained
behind an internal gate, its code/destructor lifetime protections must remain
intact (`PaxViewMacos.swift:323`); otherwise remove that isolated path coherently.
Never pass live Rust values or Tokio handles through C exports by cast.

### Mounting into a caller-owned host

Web already accepts a selector or DOM Element at `index.ts:142`; it does not yet
return an owned readiness/disposal handle. Native rendering already accepts
CoreAnimation layers (`pax-gpu/src/render_backend/mod.rs:850`) and web canvases
(`:812`). Those are useful internal seams, not a public arbitrary-window embedding
contract. Apple engine containers and cartridge management are currently shared
singletons; web rejects duplicate live mounts. Refactoring these responsibilities
into a reusable single-mount host adapter is real implementation work.

Proposed public operations:

- `init`: resolve/configure the cartridge and return a one-use prepared app;
  no window, root construction or event loop yet.
- `run`: create a default platform host, mount the prepared app using the same
  core as embedding, and run the native lifecycle. Expose useful window options
  such as title, initial/minimum size and resizability, with target restrictions.
- `mount`: consume a prepared app and attach it to an existing host view on its
  UI owner. Return an owned mount handle without entering another event loop.

Conceptual usage, not actual types/signatures:

```rust,ignore
// Standalone: main owns launch policy; the runner owns the default window.
let app = pax::init(InitOptions::default())?;
pax::native::run(app, WindowOptions::default())?;

// Existing AppKit application: caller owns parent view and event loop.
// host_view is a validated, lifetime-managed adapter for that NSView.
let app = pax::init(InitOptions::default())?;
let mount = pax::macos::mount(host_view, app)?;
// Retain mount in the view/controller. Observe readiness without blocking UI.

// Browser: invoked after JS/Wasm setup; resolves when mount is ready.
let mount = init_web(container).await?;
```

For native, `mount` returns a Mount handle while any asynchronous setup is
pending. `mount.ready()` is an awaitable result or platform callback, driven by
the existing UI loop, never a reason to block it. The web convenience awaits
the same readiness condition before resolving `init_web`. `close()` immediately
invalidates authority and initiates nonblocking teardown; `closed()` acknowledges
completion while the host continues driving cleanup. Drop starts safe teardown
but cannot synchronously guarantee completion. Final signatures must make
retention until acknowledgement concrete.

| Platform | Preferred caller-supplied mount target | Why |
| --- | --- | --- |
| macOS | Existing NSView, with safe retained/borrowed adapter and main-thread validation | Supports a region of a window, native child views, input and layer composition |
| iOS / iPadOS | UIView container plus required UIViewController/scene context | Native controls, responder chain, presentation and scene lifecycle need more than a layer |
| Browser | DOM Element container | Host can accommodate canvases and native DOM controls together |
| Future Android | View/ViewGroup and lifecycle/context adapter, with replaceable rendering surface | Activity and surface lifetimes differ; native controls require Android host integration |

This is same-process embedding into an app-owned view, not transferring a window
from a separate process. A platform window handle can help create a GPU surface,
but does not establish native control parenting, IME, accessibility, focus or
Pax composition. Do not document a raw handle as complete Pax host support.
The [raw-window-handle lifetime contract](https://docs.rs/raw-window-handle/latest/raw_window_handle/struct.WindowHandle.html)
also requires pointer validity; an arbitrary pointer must not enter a safe API
without ownership/thread validation. Native bindings may encapsulate the unsafe
conversion in a platform adapter. Retaining an object alone does not establish
every lifecycle/thread invariant.

The mount contract must cover:

1. **Ownership:** caller owns its window/parent view and loop; Pax owns only its
   child hierarchy and app session. Closing Pax must not close the caller's window
   or stop the host loop. Disposing a view revokes callbacks and retains cleanup
   separately. No use-after-free from delayed frame, task or native callbacks.
2. **Geometry and presentation:** container bounds, pixel scale, resize,
   clipping, zero-size/hidden states and native-control/vector ordering. The host
   controls placement; Pax lays out inside that region. Reparenting need not be
   promised initially; unsupported transitions must be explicit.
3. **Input and platform facilities:** pointer/touch, keyboard focus, IME,
   accessibility, clipboard and required presentation context. The supplied
   adapter provides the existing chassis behavior; missing capabilities must
   be reported, not silently replaced with inert controls. New accessibility or
   control capabilities beyond today's chassis are separately scoped.
4. **Scheduling and lifecycle:** wakeups, frames, suspend/resume, surface loss
   and recreation, errors, ready and close. Platform adapters wire these into
   the caller's loop. Embedders should not manually forward every raw platform
   event in the ordinary API. No nested loop or per-frame async polling.
5. **Application I/O:** startup data and explicit application message/event
   channels with defined boundary encoding, ownership and cancellation. No
   direct Root pointer, Property pointer or generic service lookup across FFI.

Selected first scope: standalone launch plus one embedded Pax region for
each currently supported chassis (macOS, iOS/iPadOS, web), reusing its actual
native-control integration. This includes native adapter extraction and fixtures;
it is not merely an extra `window_handle` argument. Zack confirmed delivery in
this task, rather than contract-only design. Reject a second live mount
clearly until shared host state is made per-instance. Design handles without a
permanent global API requirement, but do not promise multi-window support now.

An advanced surface-only host can later accept window/display handles or an
offscreen render target with explicit input/frame integration and capability
limits. Caller-supplied wgpu devices/queues, rendering into a host render pass,
arbitrary event-loop adapters and full multiple-mount support are separate work.
Do not silently disable native controls to claim those modes are implemented.

## 7. Selective removal and migration inventory

This is a hunk-level plan after approval, not a git revert command.

| Area | Remove or migrate | Preserve / dependency |
| --- | --- | --- |
| `pax-macro/src/lib.rs`, `src/templating.rs`, `templates/derive_pax.stpl` | `#[application(Type)]` parsing, application type selection, generated hook calls | Active-root ownership, component generation, target gates, panic boundary; replace entry plumbing only after removal checkpoint |
| Root selection and cartridge ownership | Phase one: retain `#[main]` and source layout, routing generated startup through the new initializer/root-adoption seam. Phase two: introduce explicit cartridge composition and migrate layout/root declarations | Qualified component identities, dependency isolation, normal Rust visibility and template path resolution; phase two distinguishes cartridge package from launcher package |
| `pax-runtime-api/src/application.rs` | `Application`, `EmptyApplication`, `AppBuilder::provide/provide_managed`, type registry, `AppContext::service`, service errors, hook function pointers | Instance phase/identity, graph, dispatcher, weak context, staged activation, cancellation, limits and error sink; decouple them from configuration |
| `application_shutdown.rs` | Public generic service registration/ordering contract as selected in review | Tickets, retained cleanup owners, deadline diagnostics, wake-after-view-disposal, safe owner-thread destruction; currently coupled to AppInner |
| `pax-runtime/src/api.rs`, engine/context | Registry access and obsolete lifecycle-hook routing | Node/application scopes, safe update ordering, invalidation-before-unmount, wake identities |
| `pax-tokio` | Service lookup examples/name and registration coupling | Owned/borrowed behavior, tracking, guarded result helpers, panic observation, off-UI runtime shutdown; select explicit owner/client shape |
| `pax-web-async` | BrowserService registration coupling | Local execution, staged work, scoped delivery and tracked cleanup |
| Property and rendering changes | None as a consequence of main ownership | Shared Property, LocalProperty, untyped thread affinity, mixed graph, stable snapshots, bounds and all correctness regressions |
| Apple host/build metadata | Entry selection and packaging only where necessary | Info.plist/entitlements overlays, signed-network fixes, initial frame and display-link retry, disposal fixes, native controls |
| Rust logic-reload paths in compiler, macOS host and web host | Disable supported replacement and migrate CLI diagnostics; remove isolated obsolete machinery where warranted | `.pax` reload, dev inspection, normal startup/disposal safety; any retained replacement internals keep their code-lifetime protections |
| `examples/src/async-workbench` | application.rs hook implementation, work_service configure, service lookups in root/request_panel | Same timer/I/O/cancel/remount/stream/panic scenarios and visual proof; migrate to explicit inputs/owners |
| `tests/src/tokio-support-probe` and runtime tests | Hook/registry fixtures | Thread-safety, delivery/cancellation, wake-only, shutdown, code lifetime and release checks |
| Public docs/generated references/bundles | Application setup/services guidance and examples | Shared/local property teaching, platform capability restrictions, unrelated API docs; regenerate only after implementation |

`29bab61a8` touches 247 files: a blanket revert would discard independent
correctness and platform work. `14b6f8d6a` is fixture/docs refinement, not an
architectural removal unit. Treat the old spec as historical implemented design;
link a superseding decision after approval rather than rewriting history.

Ordered removal checkpoint: align implementation checkout with agreed main;
inventory consumers; decouple lifecycle kernel and adapters from public registry;
remove hooks/lookup plus affected fixture guidance coherently; verify retained
safety/async guarantees; then add the selected main-driven replacement. If a
buildable intermediate checkpoint needs a narrow internal default constructor,
document it rather than retaining two public initialization architectures.

## 8. Implementation and validation plan after approval

### Phase one: prove entry, initialization and mounting

1. **Removal boundary:** perform section 7 before replacement implementation;
   preserve existing property, async, shutdown and Apple regression coverage.
2. **Apple launch feasibility gate:** smallest existing target fixture, Rust
   executable plus SwiftUI library bridge and separate cartridge. Verify one
   entry, cartridge-owned runtime setup, ABI compatibility and code lifetime,
   correct thread, Swift runtime linkage, app bundle/resources, signing, window,
   input and lifecycle. Start macOS, then existing iOS/iPadOS destination. If it
   fails, stop and revisit the spec; no unapproved UIKit rewrite or fake main.
3. **Typed root construction:** select library/bin target explicitly while
   retaining the current root marker and selected application package. Route
   generated startup through an ordinary initializer and typed root-adoption
   seam. Avoid duplicate roots/exports and parser execution of user main.
   Dependency components must not launch an application. Prove supplied state
   reaches first mount in debug/release without changing source layout.
4. **Web export:** invoke after module and host facilities are ready; pass mount
   input, await readiness, propagate error, return owned disposal handle; no
   implicit auto-start side effects on dependency import.
   **Embedding:** extract one-mount native view adapters; show Pax within an
   existing host view alongside host-owned controls. Verify focus, resize,
   composition, ready/failure and disposal without closing the host or adding a
   second event loop. Web fixture mounts into a supplied element. iOS embedding
   and standalone launch are separate qualification paths.
5. **Lifetime and adapter migration:** explicit runtime ownership and root inputs;
   retained scopes, cancellation and shutdown. Disable Rust replacement according
   to the selected migration; preserve `.pax` reload and diagnose unsupported
   flags. No new reload resource protocol is required.
6. **Docs and release checks:** update canonical articles, source API comments,
   affected generated docs and required example bundles; qualify shipping builds.

Phase one is complete only after standalone launch and one-region embedding
are qualified on macOS, iOS/iPadOS and web, with the async/lifetime, reload and
debug/release checks below. A macOS/web vertical slice is an early proof, not
permission to omit the already-selected iOS/iPadOS scope. Record simulator,
physical-device and signing limits explicitly; untested paths do not pass by
inference. If a platform gate fails, surface the specific constraint before
changing the agreed architecture.

### Phase two: explicit root selection and flexible source organization

After phase one passes, implement the root-selection contract in section 4:
explicit cartridge composition, ordinary reusable root components, nested
`ui/` layout and an independently selected workspace UI cartridge package.
Settle declaration spelling against the proven initializer seam; do not reopen
the host/runtime ownership contract merely to change discovery syntax.

Migrate root markers and examples with clear conflict diagnostics. Qualify
qualified type identities, module visibility, template/asset/watcher paths,
dev inspection and debug/release generation in both layouts. Update Getting
Started and Components & Composition in that phase. The phase-two declaration
and layout are not required to accept phase one.

### Pre-ship checkpoint: run workflow and examples organization

Zack requested two decisions be revisited before shipping this work, after the
initial refactor is proven. These do not block starting phase one, but must not
be lost in implementation or treated as settled by the illustrative Cargo setup.

1. **Preferred run command.** Reassess whether `pax-cli run` remains the best
   public entry, whether ordinary `cargo run` can be supported ergonomically,
   and which responsibilities belong to each. A real Rust main does not by
   itself perform Pax code generation, asset preparation, web hosting, native
   bundling/signing or reload supervision. Evaluate a fresh project with no
   generated artifacts, existing-codebase integration, native/web targets,
   argument forwarding and working-directory behavior. Choose and document the
   intended workflow before freezing starter manifests and launch wrappers;
   avoid circular Cargo-to-Pax-to-Cargo launch chains.
2. **Examples layout and curation.** Coordinate with PAX-1005 and perform any
   approved physical flattening, showcase/fixture organization and obsolete
   launcher removal in one coherent migration pass. Include phase-two source
   layout changes in the migration map where they affect the same examples.
   Keep this repository-wide path work after phase-one proof so runtime
   regressions can be distinguished from relocation errors.

The investigation checkout confirms the historical runner structure:
`examples/<name>.rs` calls `examples/scripts/mod.rs`, whose comment explicitly
describes proxying Cargo's example discovery. The helper enters
`examples/src/<name>` and runs Cargo; that package's `bin/run.rs` invokes
`./pax run`. The root workspace excludes `examples`; there is no enclosing
`examples/Cargo.toml` here. This supports reassessing the extra `src` layer and
wrappers, but does not establish that plain Cargo already builds a complete
Pax application or that the standard package-local `src/` should be removed.

PAX-1005 was read with its comments, status, labels, project and relations on
2026-10-04. It is Todo in Pax Launch, with a 2026-09-27 instruction to hold
physical migration until active website/example work lands. Recheck that state
when scheduling the migration; reading the ticket here does not start it or
change its status. Its existing contract includes:

- Propose the per-example classification and old-to-new path map before broad
  moves. Calculator and Paxflix should be prominent; useful teaching examples
  and regression fixtures remain available. `examples/<name>` plus
  `examples/fixtures/<name>` is an option, not a selected destination.
- Inventory active consumers and coordinate conflicts around ongoing example
  work. Update Cargo paths, run/create/cleanup helpers, compiler tests, CI,
  docs discovery, README links, deployment paths and source includes together.
- Update bundle/discovery source policies and regenerate their required outputs;
  pass both sync checks and validate packaged `create`, docs source access and
  representative showcase/fixture runs from final paths. Preserve starter
  eligibility/default separately from showcase prominence. No duplicate source
  trees, symlink workaround or opportunistic example rewrite.

The ship review must record the chosen run workflow and the examples migration
decision. If proceeding with physical moves, complete and validate the combined
migration before shipping; any deferral must be explicit with Zack. Resolving
the classification/path map later is deliberate sequencing, not a reason to
delay the entry/lifetime refactor now.

### Release representation audit

At `14b6f8d6a`, `pax-compiler/src/lib.rs:389` passes
`ctx.is_release && !ctx.should_run_designtime` as `use_rust_manifest`.
`cartridge_generation/mod.rs:43` calls `rust_manifest::to_rust_expression`;
`templates/cartridge_generation/cartridge.tera:26` constructs a PaxManifest.
Debug embeds JSON. The active path does not directly decode ProgramIR binary.
This matches the caveat already recorded in architecture-diagram/TOPOLOGY.md:516.

Honor the binary-baking audit requirement nevertheless: inspect `program_ir`,
`binary`, `rust_manifest`, descriptors, macro root ownership, cartridge templates
and roundtrips. Entry/factory pointers, runtime owners, closures, Tokio handles,
scope identities and live root allocations are execution state, never serialized.
Prefer direct generated Rust entry references. Any new execution metadata must
survive both active release construction and binary/IR roundtrips. No schema
change is justified merely to encode a live callback.

| Validation area | Required evidence |
| --- | --- |
| Entry | One process main/native startup; one web invocation/mount; dependency roots never launch; explicit target selection; no user startup during CLI analysis |
| Artifacts / ABI | Launcher and cartridge built once and selected explicitly; no duplicate app engine in launcher rlib; mismatched exports rejected; matching debug/release packaging; callbacks/resources keep cartridge loaded |
| Root selection, phase one | Existing root marker/package and source paths retained; typed root/initializer agreement; no duplicate exports or dependency auto-start; debug/release identity preserved |
| Layout / root selection, phase two | Nested UI modules and separate UI cartridge package; explicit declarations and conflict diagnostics; template/assets/watcher paths and debug/release identities agree |
| Embedding | Caller-owned view and loop survive disposal; sizing/scale/focus/native controls; second live mount rejected; partial startup cleanup; host view destruction during pending work |
| Initialization | Factory once, graph entered, root identity preserved, no extra default instance, target/viewport available at documented boundary, clear root/settings precedence |
| Errors | Pre-loop error, factory failure/panic, web rejection, abandoned startup, partial resource cleanup; no C unwind |
| Async | Native worker timers/TCP and dedicated current-thread owner; browser timer/fetch; no-frame-tick wake proof; UI responds during held work |
| Lifetime | Guarded stale completion, cancellation/unmount/remount, bounded streams, worker publication races, callback drop on owner, task panic sink |
| Shutdown | Borrowed runtime survives; owned runtime destruction off UI; non-abortable work retains dependencies/code; failed/discarded startup cleanup; host close versus suspension |
| Reload | Native rebuild/restart and browser rebuild/page reload; `.pax` reload retained; logic/all flag migration and overrides diagnosed; release reload disabled; test retained legacy replacement paths only if they remain enabled internally |
| Apple | Debug and signed release, entitlements retained, initial-frame failure recovery, input, scene background/resume; simulator and physical-device results separately stated |
| Browser | Debug/release, explicit mount inputs, async readiness/error, disposal and replacement, module imports without unexpected app startup |
| Release | Active Rust-manifest constructors plus ProgramIR/binary roundtrips; new root injection with literal/computed/two-way/local/shared fields |

Reuse narrow tests/caches; build targets sequentially. No new VM or simulator
is authorized. A returning macOS event loop and portable await-before-root UI
executor need additional tests if selected. Do not infer them from a Tokio probe.

Documentation impact after implementation: update Getting Started (entry/project
layout), Events and Rust (replace setup/services section and preserve async
contracts), Components (dependency ownership), How Pax Runs (startup), Developer
Workflow (restart/reload), Targets (artifacts/platform constraints), relevant
`///` comments, architecture topology and canonical examples. Review a new public
chapter outline with Zack if one becomes necessary. This draft changes no public
behavior, so it does not teach proposed APIs as already available.

## 9. Execution baseline and remaining engineering gates

The design is ready for phase-one implementation. There are no outstanding
product questions required to begin; exact API spelling and internal owner types
can be resolved against the contracts and qualification fixtures in this spec.
The selected direction and recommended engineering defaults are consolidated here:

- **Entry and linkage:** real native `fn main` on macOS and standalone
  iOS/iPadOS, plus explicit browser `init_web`. Preserve the cartridge boundary
  and current Apple cdylib packaging through a thin SwiftUI launch bridge.
  Android remains a future hosted entry; no new chassis is added.
- **Root initialization:** invoke ordinary typed construction once with the
  graph entered, preserving the supplied root's identity and state. Keep
  `#[main]` and current source organization through phase one.
- **Runtime ownership:** user code configures the exact Tokio runtime inside
  the cartridge and transfers it to a narrow lifecycle adapter. No hidden
  runtime, generic service registry or new process-to-cartridge executor ABI.
  Same-linkage borrowing retains its explicit owner-lifetime requirement.
- **Async startup:** mount the application's Loading state and update it as
  async work completes. No synchronous UI wait or portable native
  await-before-root executor is added.
- **Mounting and disposal:** deliver one-region embedding on each current
  chassis using the same mount core as standalone launch. Readiness is mounted
  and accepting work; close revokes authority immediately; completion retains
  resources/code and is driven independently of the caller awaiting it.
- **Reload and migration:** retain `.pax` reload, disable supported Rust logic
  replacement, and require native rebuild/restart or web rebuild/page reload.
  Apply the visible CLI/environment/metadata migration in section 6. Remove
  superseded public hooks/services selectively while retaining existing safety
  and async behavior; no wholesale revert.
- **Phase two:** Zack requested flexible root organization as its own phase
  after the initial refactor is proven. Then introduce explicit cartridge/root
  declarations, nested `ui/` and separate UI-package support. Preserve template
  path semantics. Phase one must expose an internal seam for this work without
  requiring the phase-two public syntax or layout migration.
- **Before shipping:** revisit the preferred run command and examples layout.
  Coordinate any approved moves with PAX-1005's showcase/fixture curation in
  one migration pass. Section 8 records the decision and validation checkpoint;
  these questions remain open for that checkpoint, not for starting phase one.

The remaining risks require implementation evidence: Apple entry/linking and
packaging, native embedding with real controls, shutdown/code retention during
pending work, and equivalent debug/release root construction. Section 8 makes
those acceptance gates. A failed gate requires reporting the concrete issue,
not substituting a synthetic main, disabling controls or weakening cleanup.

Readiness assessment is complete; implementation has not begun. The phase
ordering and pre-ship review checkpoints are recorded. Discussion of execution
readiness is not itself recorded as an instruction to start coding.

## 10. Checks performed in this investigation

Read-only source/history and installed SDK inspection, issue/project/comment
review, and primary-documentation research. No app build, behavior test, prototype,
simulator launch or signed package verification was performed.

`mdbook build pax-docs/book --dest-dir /private/tmp/pax-1015-book-check` passed.
An isolated mdBook containing this draft also rendered successfully; its local
anchors have no unresolved links. External links were
reviewed against the issue/documentation sources; this is not an automated
external-site availability guarantee. Temporary render output was removed.
Documentation checks do not qualify any proposed API or platform behavior.

## 11. Comparative assessment: Slint, GPUI and Makepad

Reviewed official documentation and upstream source on 2026-10-04, at Zack's
request before committing to the design. Slint documentation identifies 1.18.1;
GPUI observations refer to Zed's moving `main`, Makepad to moving `dev`. These
are source observations, not built/tested integrations or pinned release
guarantees. They do not approve implementation or alter selected scope.

### Findings

**Slint:** ordinary Rust main constructs generated component handles; showing a
component and running the event loop are separate operations, with `run` as a
convenience. Its Android entry instead uses `android_main`, backend init and a
cdylib. This supports explicit platform entry adapters rather than treating all
targets as executable main. Sources: [Rust API](https://docs.slint.dev/latest/docs/rust/slint/)
and [Android integration](https://docs.slint.dev/latest/docs/rust/slint/android/).

Slint's UI-thread `spawn_local` is not a Tokio executor. Its documentation
describes runtime-context/driver constraints and an `async_compat` convenience
that can initialize a shared Tokio runtime. For Pax, retain explicit user-created
Tokio ownership instead of implicitly allocating that compatibility runtime.
Its [Platform](https://docs.slint.dev/latest/docs/rust/slint/platform/trait.Platform)
and [WindowAdapter](https://docs.slint.dev/latest/docs/rust/slint/platform/trait.WindowAdapter)
define a real host boundary: renderer, event delivery, sizing and wake integration
are responsibilities beyond a window pointer. See also
[spawn_local](https://docs.slint.dev/latest/docs/rust/slint/fn.spawn_local).

**GPUI:** the current examples use an Application run callback, explicit window
creation and a typed root factory. Its platform interface separates foreground
and background executors. Its optional `gpui_tokio` module offers both a default
runtime and `init_from_handle` for an application-provided runtime. The returned
GPUI task arranges Tokio cancellation on drop; owned-runtime disposal uses
`shutdown_background`. This is useful API precedent, not an adequate cartridge
unload protocol. Sources: [window example](https://github.com/zed-industries/zed/blob/main/crates/gpui/examples/window.rs),
[platform contract](https://github.com/zed-industries/zed/blob/main/crates/gpui/src/platform.rs),
[Tokio adapter](https://github.com/zed-industries/zed/blob/main/crates/gpui_tokio/src/gpui_tokio.rs).
Its [async contexts](https://github.com/zed-industries/zed/blob/main/crates/gpui/docs/contexts.md)
make state access fallible after the window/app disappears, consistent with Pax's
existing weak-context and guarded-publication direction.

Zed's separate [embedded_gpui experiment](https://github.com/zed-industries/embedded_gpui)
hosts GPUI Wasm components inside GPUI. Its design describes a branch-level
`run_embedded` hook returning an owning ApplicationHandle when the external
platform loop returns immediately. It separates host surfaces and guest views,
geometry/input and explicit communication across the boundary. It explicitly is
not a supported API, and it is not evidence of arbitrary AppKit/UIKit embedding.
Borrow the retained-session principle without importing its plugin object model
or Wasm sandbox into this task. [Design source](https://github.com/zed-industries/embedded_gpui/blob/main/DESIGN.md).

**Makepad:** its `app_main!` macro centralizes startup and emits distinct native,
Android/JNI and Wasm creation/message entry points. A shared event-handler setup
constructs application state on Startup; current hello-world source uses a binary
entry macro, so a dual lib/bin Cargo template is not universal practice.
Sources: [entry generator](https://github.com/makepad/makepad/blob/dev/platform/src/app_main.rs)
and [example manifest](https://github.com/makepad/makepad/blob/dev/examples/hello_world/Cargo.toml).
Its Cx owns a `makepad_futures` executor/spawner and action channel, with a
`SignalToUI` path; this is a toolkit async/wake mechanism, not automatic Tokio
compatibility. Sources: [Cx](https://github.com/makepad/makepad/blob/dev/platform/src/cx.rs)
and [Cx API](https://github.com/makepad/makepad/blob/dev/platform/src/cx_api.rs).
The reviewed [GL bridge](https://github.com/makepad/makepad/blob/dev/platform/src/gl_render_bridge.rs)
lets external rendering appear inside Makepad; it does not itself establish the
reverse operation of mounting a complete Makepad app into an arbitrary native
view. This quick review did not verify such a public native-view contract.

### Recommendations for the Pax specification

1. Keep `run` as the convenient standalone composition of host creation, mount
   and native lifecycle. Implement embedding against the same mount core, with
   an owning session handle and caller-owned loop. Require a native embedding
   fixture before freezing the host API, not only at final integration.
2. Preserve explicit runtime **adoption** and same-linkage **borrowing**. Neither
   operation creates or replaces the supplied Tokio runtime. Peer APIs show why
   this is useful, but their in-process Handle examples do not solve crossing
   our cartridge ABI. Keep that distinction explicit; no hidden Tokio default.
3. Specify task-handle drop, cancellation, detach and error semantics together.
   Audit existing TaskControl behavior before changing it; do not silently copy
   GPUI's cancel-on-drop rule. App/node authority must still revoke publication
   when a task outlives its UI. Cancellation requests are not proof of completion.
4. Preserve wake-driven UI delivery and cleanup independent of frame production.
   Slint's [libuv integration](https://www.slint.dev/blog/slint-and-the-nodejs-event-loop)
   documents replacing periodic loop pumping with host-loop coordination. Test
   task completion and teardown with no animation/display ticks, as well as idle
   behavior. Do not broaden this task into a generic UI future executor.
5. Make the first embedding fixture substantive: a host-owned sibling text field,
   a Pax native control and vector content, resizing/scaling, focus transfer, and
   disposal during pending I/O. The host view and loop must remain usable after
   disposal. Raw-surface rendering alone does not qualify embedding.
6. Keep generated entry glue small and target-specific, following Makepad's
   centralized platform adaptation while retaining Pax's explicit user-authored
   entry functions. An author should see where root construction and resource
   ownership live without implementing a lifecycle/service trait.

Assessment: the selected loading-root, mounting and explicit-executor direction
stands. Preserve the cartridge boundary, while distinguishing it from a choice
to use physical dynamic linkage for every native deployment. The reviewed normal
Rust startup paths do not establish a need for a separate application cdylib.
Keeping Pax's current Apple cdylib is a Pax packaging/compatibility choice, not a
requirement of main ownership or Tokio integration. This review does not silently
add a second native deployment mode or a general-purpose plugin ABI.
