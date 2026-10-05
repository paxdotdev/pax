# PAX-1015 — User-owned main and initialization

Status: **Phases 1 and 2 implemented; phase three authorized, dependency-graph experiments expose a feature-isolation decision at the first gate. Example migration explicitly deferred.**
Investigated 2026-10-04; implementation qualification updated 2026-10-05.
The original investigation and API sketches record the approved entry design. Section
8’s implementation evidence and the phase-two qualification below distinguish delivered APIs from future work;
canonical usage is documented in [Targets](../targets-build-deploy.md#entry-and-mounting)
and [Events and Rust](../event-handling-rust.md#application-setup).
The [phase-three specification](#phase-three-cargo-integration) specifies Cargo integration
and caller-controlled packaging; its new APIs and commands are design targets,
not available capabilities.

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
No implementation was made during that investigation. Phase-one execution
subsequently began on the rebased branch, as recorded in section 9.

Both issue descriptions, all available comments, status, labels, projects and
relations were read. PAX-1015 is In Progress in Pax Core, related to PAX-1013,
with no blocking relations. PAX-1013 is Done in Pax Launch. Its complete merged
`pax-docs/book/src/design/PAX-1013-application-async-spec.md` was read; it explicitly
deferred custom main while delivering independent safety and async work.

Section 9 records the execution baseline and the two-phase scope. No further
product decision is needed to start phase one; platform feasibility and lifetime
correctness remain implementation gates, not assumed results. Zack has now authorized Phase 1 as recorded in section 9. Remove the superseded public services/custom init
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
runnable/root declarations and qualifies nested `ui/` and separate UI-package
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
preserve in-process Rust replacement; the subsequent Phase 1 authorization selects the retirement strategy below.

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

## 3. Pre-refactor startup and ownership

This section records the PAX-1013 baseline at `14b6f8d6a`, including APIs
removed by Phase 1. It is historical evidence, not current usage guidance.

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
src/lib.rs         module wiring and runnable declaration/exports
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

**Phase two, implemented after proving phase one.** Phase one retained `#[main]`,
package selection and template path resolution while changing startup plumbing
for user-owned entry and typed initialization. Explicit runnable declarations
and separate UI-package qualification belong to phase two; they were not part
of phase-one completion.

Zack raised the need to make Pax a flexible neighbor in a codebase, including
putting UI under a `ui` directory. Recommendation: separate the executable entry,
the runnable declaration and the root component's source location. None of
`main.rs`, `lib.rs`, `App`, `Main` or a directory name determines the UI root.
The layout above is a starter template, not an imposed application architecture.

At the phase-one baseline, `examples/src/increment/src/lib.rs` used `#[main]`
and the active application's marked component injected cartridge source and
platform exports. Phase two separates that ownership from the component marker.
Package identity now also accepts the explicitly selected cartridge library;
its directory need not be the parent of the host's `.pax/` directory.

Phase-two declaration (implemented; qualification is recorded below):

```rust,ignore
// src/lib.rs: library wiring; UI implementation remains under ui/.
pub mod ui;

pax_kit::runnable! {
    root: crate::ui::App,
    init: crate::ui::init,
}
```

`ui::App` is an ordinary `#[pax]` component; `ui::init` is an ordinary typed
initializer returning that root. Check their types for agreement. The declaration
provides explicit build-time root discovery and generates the native/web adapter
plumbing once for the selected cartridge. Do not discover a root by executing
main/init, scanning arbitrary runtime mount calls, guessing filenames, or requiring
the component itself to define process entry behavior. The public spelling is `pax_kit::runnable!`; its `init:` field is optional
and defaults to construction through `Default`.

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
| Root selection and cartridge ownership | Phase one: retain `#[main]` and source layout, routing generated startup through the new initializer/root-adoption seam. Phase two: introduce explicit runnable declarations and migrate layout/root declarations | Qualified component identities, dependency isolation, normal Rust visibility and template path resolution; phase two distinguishes cartridge package from launcher package |
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
explicit runnable declarations, ordinary reusable root components, nested
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

Checkpoint update, 2026-10-05: Zack explicitly agreed to defer example cleanup
and the combined PAX-1005 classification/path migration until the remaining
entry/build-workflow questions are settled. This branch retains current example
paths. The preferred run workflow remains under assessment, including plain
Cargo integration and caller-controlled packaging; retaining `pax-cli run` as
the sole canonical path has not been selected.
The [phase-three proposal](#phase-three-cargo-integration) below records the
recommended direction and the build-graph proof required before adopting it.

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
chapter outline with Zack if one becomes necessary. Phase-one implementation updates those canonical articles and generated API
references together; the older sketches in this spec are not a substitute for
the implemented signatures.

### Phase-one implementation evidence — 2026-10-05

The implementation retains `#[main]`, adds `#[init(path)]` typed root construction
and `#[pax(rust)]` dependency fields, and removes the public Application/hooks/
service registry. Root adoption preserves the supplied allocation and Property
identity. `pax-host` is a dependency-free native launcher SDK linked only to the
Swift `PaxHost` framework; the app cartridge remains separately linked. The same
native mount core serves Rust standalone entry and caller-owned native views.
Browser hosts call explicit `Pax.init_web` and own an independently closable region.
Startup inputs and request/reply use copied, bounded bytes. User-configured Tokio
and browser executors attach only lifecycle ownership to the application.

All 49 application examples now have a feature-gated standalone binary and keep
their existing library/source paths. Fresh-project creation rewrites the binary
name/default-run and target-specific dependencies correctly. Rust logic reload is
retired at the supported policy boundary (`all` migrates to `pax`; `logic` errors).
The old native replacement internals remain isolated from that supported workflow.
No example directory moves or phase-two root syntax are included.

| Area | Recorded evidence |
| --- | --- |
| Runtime/compiler | 201 compiler tests; 236 runtime tests passed, one ignored; 119 runtime-API tests, seven macro tests, ten Tokio tests. Core runtime/API/macro/Tokio checks also passed in release; the added bounded-error-reply regression passed in both modes. |
| Release representation | Seven manifest roundtrip tests passed in debug/release. Live root/closure/runtime ownership stays outside ProgramIR, binary and generated Rust-manifest data; no serialized schema was added. Workbench's baked release root retained initialized state and dependencies. |
| Web | Final debug and release Workbench builds passed. Explicit mount, failure/retry, startup cancellation, duplicate rejection, copied startup `7`/reply `8`, independent host/Pax input, async timer, 100-event stream, resize, collapse, close and remount were exercised. Import does not auto-mount. |
| Template reload | A disposable debug Workbench served with `--hot-reload=all` displayed the migration notice and live-updated a `.pax` title. Initialized input text, counter and root interaction state survived; initializer count stayed one; its executor still completed a timer. |
| macOS | Final universal release has x86_64 + arm64 slices in the Rust main, PaxHost and PaxCartridge; strict deep code-signature verification passed. Executable linkage contains PaxHost and libSystem, not another cartridge engine. Earlier interactive debug embedding and signed-release standalone checks covered real controls, timer/TCP, host survival and explicit Rust window options. |
| iOS/iPadOS | Debug and final release simulator builds passed for both target configurations, using existing iPhone 17 Pro and iPad Pro 11-inch (M5), iOS 26.4. Earlier interactive debug checks covered standalone/embedding, native text focus/input, request/reply, async interaction, duplicate/close/remount and iPhone rotation. Simulator builds deliberately disable Xcode signing; they are not device/distribution signature qualification. |
| Final native lifecycle | `tests/src/host-embedding/LifecycleProbe.swift` passed against all three final release frameworks. It verifies executor retirement after failed initialization, zero-height readiness and request delivery, resize preserving state, duplicate rejection, close acknowledgement, fresh-state replacement and close while collapsed. It never pumps an engine or shutdown queue itself. The macOS probe also passed against the final debug host and while the display clock was unavailable. |
| Native async regression | Debug/release wake-only cartridge probes passed timers, stale-result suppression, bounded stream, TCP, local callback, progress, panic reporting, blocking cleanup, discarded preparation, replacement and shutdown without periodic ticks. |
| Web host checks | All 34 JavaScript tests passed. Full TypeScript checking remains blocked by pre-existing diagnostics; comparison against HEAD found no new diagnostics. |
| Templates/docs | A newly generated project built for web with local patches to unpublished workspace crates; a fresh lockfile selects the iOS-compatible libc pin. API reference/source bundles were regenerated. mdBook and publication-link checking passed (136 pages, 11,108 links, zero errors). |

Qualification limits: physical-device signing, device lifecycle, notarization,
IME and new accessibility capabilities are not claimed. The Mac locked during
the last native visual pass. Earlier interactive native observations and the
final automated host probes are recorded separately above; the final visual
recheck of collapsed/resized native fixtures remains for an unlocked desktop.
The release web embedding fixture is available from the Workbench build output.
This is implementation/review evidence, not approval to publish or ship.

The two pre-ship decisions remain open: choose the preferred run workflow, and
coordinate any example flattening with PAX-1005's showcase/fixture organization.
Phase two (explicit runnable/root declarations and flexible UI package layout)
was subsequently authorized and implemented; its qualification is recorded below.

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
  after the initial refactor is proven. Then introduce explicit runnable/root
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

Zack explicitly authorized Phase 1 ("Okay commence with Phase 1") and rebased
the implementation branch onto `14b6f8d6a`; work continues at `c219e7ad3` plus
uncommitted implementation changes. The phase ordering and pre-ship review
checkpoints remain in effect. No further spec approval is needed for Phase 1.

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


## Phase-two implementation and qualification — 2026-10-05

Phase two is implemented. `pax_kit::runnable! { root: ..., init: ... }` owns
cartridge injection and platform adapters; `#[pax]` owns only component behavior.
The public declaration is named `runnable!` to describe a UI that can be launched
or embedded; cartridge remains the name of its compiled representation and
linkage boundary. The declaration itself does not initialize or mount the UI.
Native launcher code uses the same facade: `pax_kit::init()?.run()`.
`pax_kit::init_with` accepts explicit startup data; option, error and embedding
types remain available through `pax_kit::host`.
The launcher's `standalone` feature enables `pax-kit/host`; applications no longer
need a direct `pax-host` dependency. The implementation crate and native ABI stay
separate, and UI library builds leave the host feature disabled.
The common launcher is one ordinary, three-line `main`; unsupported-target
diagnostics live in the facade instead of repeated per-example `cfg` branches.
A shared declaration parser is used by static discovery and macro expansion.
Rust checks initializer/root agreement even when no platform adapter is active.
Legacy `#[main]`/`#[init(path)]` remains available when there is no explicit
selection; conflicting or missing roots produce a static diagnostic.

The host selects a sibling workspace library with:

```toml
[package.metadata.pax.cartridge]
package = "layout-ui"
lib = "workspace_ui"
```

The library name is an optional consistency check. Host metadata, web public
files and generated `.pax/` output stay with the host; UI source/template paths
and assets belong to the selected UI package. Native builds select the host's
standalone executable separately and use Cargo's actual target directory.
Macro ownership is tracked explicitly in Cargo fingerprints, including when a
previously active cartridge is reused as a dependency.

Static discovery resolves nested modules and explicit public re-exports.
Component identities use an accessible qualified path. Transitive reflected
types must be publicly re-exported from a direct dependency, or supplied as a
direct dependency of the consuming cartridge. Root status is assigned by the
selected program, so another library's legacy or explicit root remains a normal
subtree. Implicit lifecycle lookup now uses the dependency's module namespace.
Referenced component assets are collected; conflicting paths with different
bytes are rejected. Debug watching includes the sibling UI and referenced
source/asset directories.

`tests/src/cartridge-layouts` qualifies a host with no Rust UI dependency, a UI
library with a custom target name, nested private modules exposed through public
re-exports, a legacy-root component library with another custom target name, and
a second cartridge that reuses the explicit UI root. The 49 repository examples
now select their roots with explicit `runnable!` declarations. Example directories and launch wrappers
have not been reorganized.

Validation recorded for this phase:

- Compiler: 208 unit tests; macro: 8; manifest: 13; CLI: 28. Selection diagnostics,
  workspace metadata, watch roots, asset collisions, and rich/IR/binary/Rust
  manifest identity and root-role preservation are covered. A separate compile
  check rejects an initializer returning the wrong root type.
- Workspace web builds in debug and release; the release reuse cartridge mounts
  and both root and dependency buttons respond. The dependency's implicit mount
  handler runs. Both UI and dependency image assets are served successfully.
- Live edits to the sibling UI template and the dependency template reload;
  the initialized root retains its state. Dev inspection reports the qualified
  component identities and template paths.
- macOS debug and universal release builds, plus native lifecycle probes in both
  modes. The final release executable and both frameworks contain arm64 and
  x86_64 slices and pass `codesign --verify --deep --strict`. iPhone simulator
  debug and iPad simulator release builds and lifecycle
  probes pass. Probes cover invalid initialization, zero-height ready, duplicate
  mount rejection, byte requests/replies, resize, close acknowledgement, and remount.
- A freshly generated Increment project, with dependencies pointed at this local
  branch, builds for web and macOS without pre-existing generated Pax output.
- AsyncWorkbench builds for release web with its typed initializer and executor
  resources, using the explicit runnable declaration.
- The broader manifest integration suite has 49 passing tests and three existing
  failures: `test_default_route_metadata_cannot_be_indexable`,
  `test_route_metadata_rejects_duplicate_fields`, and
  `test_type_qualified_object_constructor_remains_supported`. An isolated build
  of unchanged HEAD reproduces all three; their route/object parser expectations
  are outside this phase. The lifecycle integration cases pass.
- Canonical articles, README and CLI help are updated. API reference regeneration,
  bundled example synchronization, mdBook build and publication-link checks run.

No physical-device, distribution signing or notarization claim is added by these
checks. Phase one's executor/lifetime contract is unchanged. The pre-ship
run-command decision and PAX-1005 example classification/path migration remain
explicitly deferred to the checkpoint above. Git commits remain Zack's.


### Application matrix spot checks — 2026-10-05

A follow-up using Calculator, Paxflix and Living Quilt found an ordinary
single-package launcher regression missed by the workspace fixture. Enabling
`pax-kit/host` also reaches the library Cargo builds alongside that package's
binary. Passing `-L framework=...` only after `cargo rustc --` left that cdylib
unable to find PaxHost. The launcher now supplies `PAX_HOST_FRAMEWORK_DIR` to
`pax-host`'s build script, which propagates the framework search path to all
consuming targets. All three macOS debug builds and the selected mobile debug
and release builds below pass with this correction. The packaged Calculator
cartridge has no PaxHost dependency; the launcher links PaxHost.

| Application | Web release: full-page / embedded | macOS debug: standalone / embedded | Mobile spot check: standalone / embedded |
| --- | --- | --- | --- |
| Calculator | Rendered; keypad `2+3=5`; embedded resize, close, host counter and remount pass | Launch and keypad arithmetic pass; corrected initial graph centers in both hosts; embedded zoom, drag, resize, collapse/restore and remount pass | iPhone 17 Pro, iOS 26.4, debug: centered graph renders; API lifecycle probe passes; Zack confirms taps work |
| Paxflix | Artwork and detail interaction pass; embedded resize, close, host counter and remount pass | Artwork and detail interaction pass; embedded resize, close, host counter and remount pass | iPad Pro 11-inch (M5), iOS 26.4, release: artwork renders; API lifecycle probe passes |
| Living Quilt | Rendering and click ripple observed; embedded resize, close, host counter and remount pass | Animation and click ripple observed; embedded resize, close, host counter and remount pass | iPhone 17 Pro, iOS 26.4, release: renders; API lifecycle probe passes |

Direct calls to the packaged macOS executables launched Calculator and Paxflix.
`pax-cli run` launched Living Quilt on macOS, Calculator on web (debug), Paxflix
on iPad, and Living Quilt on iPhone. Calculator's iPhone build was separately
installed and launched with `simctl`. This is coverage across the requested
axes, not every Cartesian combination of application, target, build profile,
host mode and launch mechanism. A browser has no direct native executable;
simulator apps use installation and platform launch. Android remains
unimplemented, and physical devices were not tested.

The native separate-host checks use the Swift fixture and public host ABI.
The new lifecycle-probe `--sample` mode avoids requiring Workbench's custom
initializer and request protocol. It checks duplicate rejection, readiness at
zero height, expansion, continued host-loop execution, close acknowledgement,
and remount followed by collapsed close. Automated mobile touch interaction is
unverified: the computer-use tool captured device windows but rejected tap coordinates with
`windowNotFoundAtPosition`. This is an automation limitation, not a successful
touch check. Zack subsequently confirmed Calculator clicks on macOS and taps
on iPhone, and exposed the then-missing Apple keyboard bridge. The follow-up
below implements and qualifies that capability separately.

The Calculator macOS graph initially opened near x=-256, y=256 and reported
`NO RESOLVED CURVE` in both standalone and caller-owned hosts. AppKit's
backing-pixel alignment of fractional placement emitted bounds notifications
before the scroll viewport's initial layout; Pax misclassified those as native
scroll input and replaced the authored center. Geometry updates now suppress
that feedback, empty/uninitialized viewports and size-only notifications are
ignored, and requested positions survive temporary content clamping. The
corrected graph is visually verified in both host modes. All 48 shared Swift
tests pass; four focused scroller regressions also pass with optimization,
including fractional placement, content shrink/growth, offsets arriving before
geometry, and continued native scroll reporting. This does not establish a
physical-device pass. Native keyboard qualification follows below.

### Native keyboard bridge — 2026-10-05

Apple canvases now forward native first-responder key events to the existing
global `KeyDown`, `KeyUp`, and `KeyPress` handlers. Standalone hosts initially
focus the canvas; embedded mounts preserve host focus until a canvas click/tap.
Native text controls retain their platform input. Handled keys return
default-prevention status to AppKit/UIKit; unhandled keys continue through the
native responder chain. Held keys release on focus loss, deactivation and close.
This adds neither IME composition nor a software keyboard for canvas editors.

`pax_interrupt` now returns `bool`, so the internal cartridge ABI advances to 2;
hosts reject older cartridges and must be rebuilt with them. The public host
embedding ABI remains 1. Existing keyboard bindings and binary-baked handler
metadata are unchanged.

Qualification: all 52 shared Swift tests pass, including four keyboard tests
that also pass with optimization. The native Calculator probe checks the exact
Swift modifier payload, cancellation, and `6+2=8` through both debug and baked
release cartridge ABIs. Real keyboard input also produces `6+2=8` on macOS
(debug and release) and the iPhone 17 Pro iOS 26.4
simulator; iPhone arrow/Backspace editing works. A separate macOS host verifies
initial host text focus, canvas/scroller focus, return to the host editor, close
and remount. Zack subsequently completed the same manual script on the iPhone
simulator's debug embedding fixture: host text input stays isolated, canvas
keyboard arithmetic works, focus returns to the host, close/remount preserves
the host, and background/resume while holding an arrow key leaves input usable.
Physical-device keyboards and iOS repeat timing remain unverified.

The updated ABI-2 Calculator also builds for the iPad simulator in release
mode. Its separate-host fixture reaches Ready and renders the graph on the
iPad Pro 11-inch (M5), iOS 26.4; it is available for the same manual script.
This launch observation is not a claim that the release interaction script has
already been completed.

<a id="phase-three-cargo-integration"></a>

## Phase three — Cargo integration and caller-controlled packaging

**Proposed and authorized for execution 2026-10-05.** Zack approved proceeding
after reviewing this specification. Start with the dependency-graph proof below;
the remaining stages depend on that result. This phase addresses the run-workflow
checkpoint and does not start PAX-1005. Existing phase-one/two runtime APIs and
qualification remain the baseline; proposed APIs are not delivered until qualified.

### Outcome and scope

Make Pax usable through ordinary Rust build tools, with a versioned Rust API for
the steps Cargo does not perform. The recommended default for a new macOS app
is a real `cargo run`, from a checkout with no generated Pax artifacts and no
globally installed Pax CLI. The resulting executable must also launch directly
and under a debugger in its development output tree. A project can separately
package its artifacts, select a simulator, sign an application or serve a web
build using its own tooling.

`pax-cli` remains the convenient frontend for target shortcuts, development
watching, inspection, serving and deployment preparation. It should use the
same build implementation as project-owned tooling. The objective is control
over the build and its outputs; adding a `cargo pax` spelling alone would not
meet it.

This phase covers the existing macOS, web, iOS and iPadOS backends. macOS is the
first plain-Cargo build/run target. Web and mobile receive composable build and
packaging paths with explicit platform launch steps. This does not add Android,
Windows or Linux hosts, restore Rust logic reload, change application lifetime
or executor ownership, or add a dependency/service registry. Example directory
flattening and showcase/fixture curation stay deferred to PAX-1005.

### Constraints established by the current implementation

The work is larger than moving `perform_build` into `build.rs`:

- `pax_compiler::perform_build(&RunContext)` is already a Rust entry point, but
  combines source preparation, compilation, packaging and optional launch.
  `prepare_cartridge_sources` also prepares interfaces, resources and metadata;
  it is not yet a standalone code-generation API.
- Apple builds compile the UI library as a cartridge, build the Swift host,
  then compile the Rust launcher with `PAX_BUILD_CARTRIDGE=false`. The same
  package's implicit library compilation during a binary build is significant:
  phase-two testing already found a framework-linking failure there.
- Generated code currently comes from `PAX_DIR/cartridge.partial.rs`.
  `pax-host/build.rs` only propagates a supplied framework search path; it does
  not build the framework. Native resource discovery assumes an application
  bundle, so linking a bare Cargo binary is not sufficient to make it runnable.
- Web uses `wasm-pack build`, followed by optimization and web/resource assembly.
  Producing a `.wasm` file alone does not produce the browser application.

Cargo build scripts execute before their package compiles; their generated
files belong in `OUT_DIR`. They can generate Rust and build native prerequisites,
but cannot consume that package's completed executable as a packaging hook.
Target configuration must come from Cargo's target environment, independently
of the machine executing the script. These are the relevant
[Cargo build-script contracts](https://doc.rust-lang.org/cargo/reference/build-scripts.html).

Cargo's explicit dependencies on compiled `cdylib` artifacts still require
unstable `-Z bindeps`, as checked on 2026-10-05. This proposal requires stable
Cargo; it cannot assume that facility is available.
See [Cargo artifact dependencies](https://doc.rust-lang.org/cargo/reference/unstable.html#artifact-dependencies).

### First gate: prove the Cargo dependency graph

Before freezing the helper API or changing starter defaults, build a minimal
macOS proof from an empty target directory. Keep the native cartridge as a real
separate ABI artifact. Draw the dependency graph and show precisely which step
produces generated Rust, the cartridge, the host framework, the launcher and
resources, and which step makes each artifact discoverable.

The preferred graph is:

```text
Pax/Rust sources + resolved project configuration
    -> source generation + native host prerequisites
    -> Cargo compilation of cartridge and launcher
    -> directly runnable development artifacts
    -> optional application packaging -> optional signing/install/launch
```

The arrow between cartridge and launcher compilation must represent an actual
supported Cargo dependency or an explicit orchestration step, not assumed
target ordering. In particular, establish whether stable Cargo can produce
and locate the separate cartridge during an ordinary single-package binary
build. Exercise both the package containing its own UI and the existing
separate-host/separate-UI fixture before choosing how workspace builds work.

Required invariants:

1. No build script invokes Cargo to build the application or its workspace, and
   no application startup compiles missing artifacts or shells into `pax-cli`.
   A separate target directory is not a justification for recursive builds.
2. The launcher retains user-owned `main`. It does not become a wrapper that
   builds and launches a different executable. The host/cartridge ABI remains
   real; linking the UI's `rlib` into the host must not silently replace it.
3. Cartridge exports, root selection and init ownership remain correct when
   Cargo also builds an `rlib`, test harness or dependency copy. Inspect linked
   symbols and loaded images for unintended cartridge exports and additional
   engine instances; do not rely on unused code being discarded by the linker.
4. No preexisting `.pax`, environment prepared by a previous CLI run, unstable
   artifact dependencies, global CLI installation or repository-relative path
   is required. Test a small project outside the monorepo as well.

If stable Cargo cannot meet these invariants with the current package layout,
return with the concrete graph and measured alternatives before widening the
implementation. Compare a deliberate launcher/cartridge package split against
an explicit project-owned build task. Do not silently sacrifice the cartridge,
require nightly, or label an `xtask` wrapper as completed plain `cargo run`
support. The reusable build API can still be valuable, but that is a distinct
partial result and requires an explicit scope decision.

### Developer-facing setup and defaults

Keep the runtime-facing API unchanged:

```rust
fn main() -> Result<(), pax_kit::host::Error> {
    pax_kit::init()?.run()
}
```

The proposed build dependency is `pax-build`, versioned with the Pax toolchain
and resolved through the project's Cargo lockfile. It is a build-time dependency;
application code continues to use `pax_kit`. Do not pull the compiler, server or
packaging tools into the application's runtime facade to save a manifest entry.
The intended simple build script is:

```rust,ignore
// Proposed build.rs API; not implemented.
fn main() {
    pax_build::compile().expect("Pax compilation failed");
}
```

Here `compile` means prepare generated code and native prerequisites for the
enclosing Cargo compilation. It does not compile the application with a nested
Cargo command. Final naming and signatures follow the graph proof. The advanced
form accepts explicit configuration and returns structured outputs/errors,
instead of requiring callers to synthesize undocumented environment variables.

Subject to that proof, recommend these starter defaults:

- Preserve `[lib] crate-type = ["rlib", "cdylib"]`, the real application binary,
  `package.default-run`, and `required-features = ["standalone"]` on that binary.
- Enable `standalone = ["pax-kit/host"]` by default **in application starters**,
  so macOS `cargo run` needs no extra flag. Reusable UI/component libraries do
  not enable it by default. This changes today's starter default and therefore
  needs an explicit migration note and CLI compatibility test.
- A consumer reusing an application library disables its default features.
  Web and cartridge-only invocations select `--lib --no-default-features` and
  the desired UI features. The build plan must keep launcher-only features out
  of the cartridge compilation. A build script cannot undo Cargo feature
  unification; failure to separate these roles is a graph-gate failure.
- Preserve the explicit `runnable!` root and
  `[package.metadata.pax.cartridge]` selection. Do not infer a UI package from
  whichever manifest happens to be the current directory. Nested `ui` modules,
  custom library names and shared components remain supported.

These are intended commands after implementation, not commands supported by the
current checkout:

```sh
# macOS application starter, with platform SDK prerequisites installed
cargo check
cargo run
cargo run --release -- --application-argument
cargo build --release
./target/release/my_app --application-argument

# Library/cartridge compilation; packaging and launch are separate
cargo build --lib --no-default-features --target wasm32-unknown-unknown
```

The direct executable example assumes Cargo's default target directory and no
explicit target triple; report the real artifact path when either is changed.
The exact web feature arguments must be established by the prototype, including
migration from existing `web`/`macos`/`ios` feature forwarding. Do not document
the abbreviated web command as working until target inference handles it.

### Target, feature and configuration ownership

| Concern | Proposed owner and behavior |
| --- | --- |
| Rust ABI / compilation target | Cargo `--target`, `CARGO_BUILD_TARGET`, or Cargo configuration; the helper reads the resolved target. Without an override, use Cargo's host target. |
| Standalone launcher vs embedded UI | Cargo capabilities and selected targets; `standalone` enables host entry. Disabling it does not change the platform or mount the UI automatically. |
| iPhone vs iPad; simulator vs device | Rust target selects simulator/device ABI; platform build options select device family and launch destination. iPad is not another Rust target triple. |
| App identity, assets and default packaging settings | Existing Cargo Pax metadata, extended only where necessary. Host metadata owns packaging; the selected UI package owns its templates and assets. |
| One invocation's packaging, signing, launch and serving | Explicit Rust API/task options; documented environment overrides may support CI. No requirement to commit local simulator IDs or signing credentials. |
| Application arguments | Everything after `cargo run --` reaches the user's main unchanged. Pax build/packaging options are not intercepted from application arguments. |

For Pax-owned settings, precedence is explicit invocation options, documented
environment overrides, project metadata, then platform defaults. The resolved
plan identifies the source of each setting. This precedence cannot override
the enclosing Cargo invocation's target or feature resolution. Universal macOS
packaging is an explicit composition of matching per-architecture builds;
ordinary host-target `cargo run` need not build a second architecture.

An environment variable naming a Pax platform cannot retarget an already-running
Cargo compilation. Reject conflicting target selections with a useful error.
Avoid mutually exclusive platform features as the primary interface: Cargo
features are unified and intended to be additive.
See [Cargo feature unification](https://doc.rust-lang.org/cargo/reference/features.html#feature-unification).

Cargo's ordinary package/binary selection, target directory, compiler flags and
profiles remain authoritative. Do not silently apply today's web size-oriented
profile overrides to a caller-owned Cargo build; expose optimization as an
explicit preset. `dev` uses the development representation and `release` uses
the baked representation. Specify and test an explicit representation choice
for custom profiles; optimization level alone does not identify a release build.
Keep the chosen representation consistent through generation, compilation and
packaging. Release reload restrictions remain intact.

### Build API boundaries

Extract one implementation used by the Cargo helper, CLI and project-owned
tasks. Start from the existing compiler rather than creating a second generator.
The following are operation contracts, not finalized public type names:

| Operation | Inputs and outputs | Permitted effects |
| --- | --- | --- |
| Resolve/plan | Host and cartridge identities, Cargo configuration, platform settings → validated inputs, steps, output locations and tool requirements | Read configuration; provide an inspectable plan without building or launching. |
| Prepare | Resolved inputs → generated Rust, baked/debug program data, prepared resources and native host prerequisites | Run source generation and required native compilation; write only declared build outputs. Safe to invoke from a Cargo build script. |
| Compile | Prepared plan → actual Rust/Swift/Wasm artifacts and provenance | An outer project task or CLI may invoke Cargo/toolchain processes. This operation is unavailable inside the build-script adapter. |
| Package | Explicit compiled artifacts and package options → app bundle, embedding payload or web directory | Assemble resources and bindings; perform selected post-compilation optimization. No automatic install or launch. |
| Sign/install/launch/serve | Explicit package and destination/options → process or session handle | Separate opt-in operations, so an external bundler/server/deployer can replace each one. |

The build-script entry uses only the preparation layer and emits the necessary
Cargo directives. Keep dev servers, simulator control and watch orchestration
out of its default dependency graph. A project-owned Rust `xtask` can depend on
the orchestration API and call all stages sequentially, without an installed
CLI. Expose artifact paths and errors rather than forcing callers to parse
colored CLI output. Long-running operations provide cancellation and ownership
of their child processes; building one package must not terminate unrelated
applications or watchers.

Shelling into a pinned `pax-cli` is an acceptable documented transition recipe
for an existing project task. It is not the build-script implementation or the
long-term integration API. No generated Cargo wrapper may call back into itself.

### Generated artifacts, resources and incremental behavior

Cargo integration writes generated source and prerequisites under its assigned
`OUT_DIR`, with tracked inputs and atomic replacement of complete generations.
It does not rewrite source files or require generated `.pax` content in version
control. Keep the current CLI output layout working through the transition;
make final packaging output an explicit option rather than a source-location
assumption.

Track Rust/Pax source inputs, resolved component dependencies, additions/removals
in relevant source and asset directories, metadata, enabled features and target
configuration. Keep source analysis consistent with the enclosing Cargo
resolution, including workspace dependencies, lockfile and feature selection.
Any metadata subprocess must respect locked/offline operation and must not
create another compilation cycle. Generation must not initialize the app,
Tokio, a GPU, or a host event loop.

Expose an artifact descriptor recording package/root identity, target, build
configuration, representation, ABI versions, resource locations and the actual
produced files. Its schema starts as an internal versioned interchange format;
do not promise a permanently stable cache format. Reject stale or incompatible
host/cartridge combinations rather than selecting a nearby file by name. Keep
simultaneous target/profile/package builds isolated. Record native SDK/tool
inputs needed to invalidate their artifacts. Respect `cargo clean`; do not
create unbounded parallel caches outside Cargo's ownership.

On macOS, implement explicit resource/cartridge resolution for both development
outputs and packaged applications. Development artifacts can refer to their
build tree and need not be independently distributable; they must launch from
another working directory without Cargo's injected dynamic-library environment.
Packaged applications must use relocatable bundle resources and run after being
copied away from the checkout. Shared native code must locate the resources of
the Pax mount, including in another application's bundle; relying solely on
`Bundle.main` does not satisfy embedding. Preserve existing ABI consumers or
version any required ABI extension explicitly.

No build script downloads fonts, installs toolchains, starts a server, watches
files, signs for distribution or installs on a device. Cargo may fetch declared
dependencies normally; other provisioning is an explicit preparatory operation
with actionable missing-tool/asset diagnostics. A warm, offline build must work
with those declared prerequisites present.

### Platform workflows and development sessions

| Platform / use | Compilation and execution contract |
| --- | --- |
| macOS standalone | Cold `cargo run` executes the user's actual binary; `cargo build` leaves directly runnable development outputs. Bundle/signing operations remain separately callable. |
| macOS embedded | Build the UI cartridge and embedding payload without launching. A separate host uses the existing mounting/lifetime API. Its build task may orchestrate the stages explicitly; do not assume a Cargo Rust dependency schedules the `cdylib` artifact. |
| Web standalone or embedded | Cargo builds the Wasm library. Post-compilation tooling generates matching JS bindings, assembles the web interface/resources and optionally optimizes. Callers can use their own server and DOM mount. This phase does not promise `cargo run --target wasm32-unknown-unknown` for the library target. |
| iOS/iPadOS standalone | Cargo builds for the selected Apple target; explicit packaging, signing as required, installation and platform launch follow. The ordinary Rust main remains the native entry, but a bare mobile executable is not a desktop-runnable program. |
| iOS/iPadOS embedded | Produce a version-matched embedding payload and resources for the caller's application. Do not impose a standalone main or launch that application during compilation. |
| Android | Unsupported backend; clear diagnostic. Preserve the architectural boundary for future integration without claiming an Android build or host implementation. |

A Cargo target runner may provide an optional mobile package/install/launch
adapter **after** the executable exists. It cannot supply missing pre-compilation
code generation. Account for runners also applying to tests/benchmarks; do not
accidentally launch the app for a test harness. Runner setup must work from a
workspace root and compose with an existing caller-owned runner.
See [Cargo target runners](https://doc.rust-lang.org/cargo/reference/config.html#targettriplerunner).
An optional runner is not a requirement for ordinary macOS development execution.

Plain `cargo run` is a build-and-run operation with no automatic watch session.
`pax-cli run` retains the current debug Pax-template reload experience. A Rust
task may opt into the same supervisor separately; do not start it from
`build.rs` or implicitly inside `init`. Application-logic changes still require
a rebuild/restart, and release builds disable reload. Documentation must explain
this difference without implying that Cargo itself watches `.pax` files.

### Delivery sequence and acceptance

1. **Prove the graph.** Produce the minimal cold macOS experiment, linked-image
   inspection and separate-package analysis described above. Measure build
   time/output size and record remaining constraints before finalizing manifests
   or the public helper API. This is the stop-and-reassess gate if stable Cargo
   cannot preserve the boundary.
2. **Extract preparation and orchestration.** Split the existing pipeline into
   the stated operations and demonstrate a project-owned Rust task on each
   supported backend. Keep existing CLI entry points working against those same
   operations. Audit the debug and binary-baked release generation paths.
3. **Deliver the macOS Cargo path.** Add the build helper, proven starter setup,
   development resource discovery and ordinary check/build/run/debugger support.
   Migrate a focused canonical example and the project template first, without
   moving the example tree or removing its historical wrappers in bulk.
4. **Qualify and document platform recipes.** Exercise web packaging and mobile
   standalone/embedding through the Rust API, plus CLI compatibility. Add the
   optional runner only if it makes those recipes simpler and satisfies its
   test-harness contract. Record the selected workflow, then revisit PAX-1005.

Required acceptance evidence:

- Fresh single-package starter and separate-package fixture, with no `.pax` or
  preexisting application artifacts, stable Cargo and no global Pax CLI. Test
  workspace-root invocation, a package outside the repository, custom library
  names, explicit root selection and feature isolation. State precisely which
  recipes use plain Cargo and which use the outer task.
- `cargo check` and rust-analyzer's normal check path diagnose changed Pax/Rust
  sources without launching anything. Ordinary library tests/doctests do not
  initialize the application. Missing native prerequisites report their remedy.
- macOS debug/release: `cargo run`, direct executable, debugger launch, arguments,
  exit behavior and working directory. Repeat with explicit target and target
  directory. Copy a packaged app away from the checkout and launch it. Preserve
  the CLI's universal-build path and verify matching cartridge/host slices.
- Correct incremental rebuilds after template, Rust, asset, dependency and
  metadata changes; deleted assets do not remain in output. An unchanged warm
  build avoids unnecessary generation/native compilation. Verify locked/offline
  operation after provisioning, concurrent target isolation and a custom profile
  with explicit representation selection.
- Use Calculator, Paxflix and Living Quilt for representative output/resource
  coverage. All three get macOS debug standalone Cargo/direct-binary checks and
  separate-host checks. Calculator additionally gets release Cargo and relocated
  bundle checks. On web, check standalone and separate-DOM-host mounts for all
  three, with at least one debug and one baked release build. On mobile, check
  Calculator debug on iPhone and a resource-rich release app on iPad in both
  standalone and separate-host modes. Confirm input, resize, close/remount and
  continued host responsiveness where applicable; retain the native keyboard
  focus regression. Record actual invokers and targets, not a Cartesian-matrix
  claim inferred from a few successful launches.
- Existing `pax-cli run` still builds, packages, launches and reloads templates on
  web/macOS and the supported mobile path. CLI and Rust-task outputs agree on
  root, metadata, assets, representation and ABI. Release artifacts have no
  enabled reload lane. Android and untested physical-device behavior stay marked
  unsupported/unverified rather than included in pass counts.

For implementation, update Getting Started, Targets/Build/Deployment, Developer
Workflow, `pax-kit`/build-helper API docs, CLI help and starter/example guidance
together. Teach the short default path first, then explicit packaging and host
integration. Regenerate affected API/example artifacts, build the book and check
links. This specification-only change does not alter current public instructions
or assert that the proposed helper exists.

Phase three is complete when the macOS Cargo path and the composable platform
recipes pass these checks, the CLI uses the shared implementation, and the
pre-ship run-workflow choice is recorded. A Rust task alone, a warm build that
depends on earlier CLI output, or a renamed CLI command is not completion.

### Phase-three graph experiments — 2026-10-05

These are prototype observations, not delivered build-helper APIs or a completed
qualification matrix. Experiments used stable Rust/Cargo 1.93.0 and Xcode 26.4
on Apple Silicon. They leave starter defaults and the approved runtime API
unchanged. The feature-isolation requirement above needs a decision before
the single-package implementation can proceed.

**Single-package artifact ordering works.** A fresh, dependency-free Cargo
fixture with `[lib] crate-type = ["cdylib", "rlib"]` and a binary built both
library forms before the binary during `cargo run`. The verbose command gave
the binary an implicit `--extern graph_ui=…/libgraph_ui.rlib` dependency. The
dynamic library was in `target/debug/deps`; the top-level copy was absent in
this run. The executable loaded the dynamic library and called its exported
entry point. That first minimal build took 0.77 seconds; a subsequent fresh
fixture with a native host framework and binary-specific link directives took
1.68 seconds. These small timings are graph tests, not Pax build benchmarks.

**Cargo feature sets are not isolated per target.** The same verbose output
passed `feature="standalone"` to both the library and binary. Forwarding it to
the facade's `host` feature enabled the host dependency for both. Moving the
framework link directive from the shared host crate to the application's
`cargo:rustc-link-arg-bin` directives prevented the cartridge from depending
on that framework, but did not remove the host feature from its compilation.
`required-features` gates whether Cargo selects a binary; it does not scope
those features to that binary. Thus the literal requirement to exclude
launcher-only Cargo features fails for the proposed single-package layout.

**A real native Pax prototype mounted successfully.** An external package used
the existing source analyzer and cartridge generator from `build.rs`, followed
by an Xcode build of the real Swift host. A header-only Swift package supplied
the cartridge's C declarations, so building the host did not require an
already-compiled Rust cartridge. Cargo then compiled the application's dynamic
library and ordinary user-owned `main`:

```text
build.rs: analyze/generate + compile Swift host against C declarations
    → Cargo: application library (cdylib + rlib)
    → Cargo: application binary, linked to PaxHost only
    → run: PaxHost dynamically loads the sibling cartridge
```

The prototype's native host and generated data lived under Cargo's `OUT_DIR`.
It used an explicit framework rpath, a cartridge path relative to the running
executable, and the Swift resource bundle copied into the host framework.
After deleting all `PAX_*`, `CARGO_*`, and `DYLD_*` variables from the launch
environment and changing to `/private/tmp`, direct binary invocation printed
`PASS Cargo proof root mounted` from the actual root's mount handler. No Cargo
compilation or CLI invocation occurred at runtime. An embedded Info.plist was
also tested, but the computer-use app selector could not attach to the bare
executable, so this is lifecycle evidence rather than a visual interaction pass.

`otool -L` showed the executable depending on `PaxHost`, and no `PaxHost`
dependency in the cartridge. `nm -gU` found the cartridge ABI exports
(`pax_init`, `pax_tick`, `pax_interrupt`, `pax_dealloc_engine`, and
`pax_cartridge_abi_version`) in the cartridge and not in the executable.
The prototype generated baked program data with a debug Cargo profile to
exercise the graph; this is not release-profile qualification. It also reused
the workstation's dependency build cache. A complete cold Pax build, runtime
image/instance audit, production root/reuse matrix, and all other acceptance
checks remain outstanding.

**Separate-package selection remains an orchestration problem.** The existing
`cartridge-layouts/host` selects its UI through Pax metadata and has no Rust
dependency on it; Cargo does not schedule artifacts from arbitrary metadata.
A small separate-host analogue with an ordinary Rust dependency did schedule
the UI's dynamic library, but its root macro's `CARGO_PRIMARY_PACKAGE` guard
suppressed cartridge entry points. The host failed its symbol lookup. This
does not prove every split-package design impossible; it shows that adding a
dependency is insufficient, and changing root selection requires another
reuse/duplicate-export qualification. The synthetic consumer-alone and
workspace builds succeeded, but that is not a production Pax reuse pass.

The alternatives at this gate are:

| Approach | Evidence and consequence |
| --- | --- |
| Keep one package; require native linkage/runtime separation, while allowing shared Cargo features | The native prototype mounts through a real cartridge. Keeps the simple starter and ordinary `cargo run`. Requires explicitly revising the feature-isolation sentence, then proving the remaining root, instance, incremental, and target cases. Recommended for the intended ergonomics. |
| Separate launcher and cartridge packages | Can give the packages different feature requests, but ordinary workspace feature unification still needs care. Adds manifest/layout overhead and still needs an explicit artifact/root-selection recipe; not a proven drop-in solution. |
| Keep the current layout and use a project-owned Rust build task | Separate build invocations can retain the existing strict launcher/cartridge feature distinction. Preserves the cartridge and exposes caller control, but does not fulfill the requested single-command plain `cargo run` path. Requires a scope decision, as stated by the first gate. |

The spec explicitly says to “return with the concrete graph and measured
alternatives before widening the implementation” if the graph cannot satisfy
its invariants. The open decision is feature isolation versus artifact/runtime
isolation, not whether to erase the cartridge boundary. No alternative may
silently substitute a statically mounted UI, require nightly Cargo, or compile
the application recursively from its build script.
