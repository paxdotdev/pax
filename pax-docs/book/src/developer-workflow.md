# Developer Workflow and Tools
<!-- summary: Build a reliable edit, reload, inspect, and capture loop with the Pax CLI. -->
<!-- tags: workflow, cli, hot-reload, inspection, screenshots, logs, formatting, debugging -->

Once your project is running, most interface work follows a short loop: change
the source, see the result, try an interaction, and inspect anything you cannot
explain from the screen alone. Pax's development tools let you move between
the running interface and the source that produced it.

Start with a project from [Getting Started](getting-started.md). Commands in
this chapter run from that project's directory unless stated otherwise. The
web examples need an open browser tab connected to the running application.


## Hot reloading

Debug `pax-cli run` sessions use **Pax-only hot reload** by default. Direct native
`cargo run` does not start a development server. Saving a `.pax` template in a
CLI session can update the mounted interface. Rust application changes require rebuilding
and restarting on native targets, or rebuilding and reloading the browser page.
The initializer runs once per application instance, including after a restart.

| Mode | Saved `.pax` changes | Saved Rust changes |
| --- | --- | --- |
| `pax` — default | Reload | Rebuild and restart manually |
| `off` | Wait for restart | Rebuild and restart manually |
| `all` — legacy | Reload, with a migration notice | Rebuild and restart manually |
| `logic` — retired | Rejected with a migration diagnostic | Rebuild and restart manually |

```sh
pax-cli run --target web --hot-reload=pax
pax-cli run --target web --hot-reload=off
```

### Target support and restart boundaries

macOS, iOS, iPadOS and web use the same policy. `all` is accepted as a migration
alias for `pax`; it no longer replaces Rust code in the running process. No
automatic restart is implied. Inline templates in `.rs` files also require a
rebuild. A new component type or event handler may need compiled information
that the running application lacks; follow the reload diagnostic and restart.

Template reload preserves the initialized root and its dependencies. Affected
nodes can still unmount and remount, revoking their scopes and recreating local
stores. Focus, scroll position and arbitrary node-local state need not survive.
Connecting to the development server with an unchanged manifest does not reload
the scene. If the server has newer template content than the built cartridge,
the initial synchronization applies that content through template reload.
See [provider lifetime rules](components-composition.md#lifetime-and-lookup-errors).
Disabling reload leaves source edits intact for the next build and preserves dev
inspection. Release builds always disable reload and dev replacement.

### Configure a project default

```toml
[package.metadata.pax.dev]
hot_reload = "pax"
```

Selection precedence is the `--hot-reload` flag, then `PAX_HOT_RELOAD`, then Cargo
metadata, then the `pax` debug default. The same migration rules apply to every
source of the setting. The CLI owns development orchestration. Both `init!` and
`init_with!` initialize in the current native process; enabling `designtime`
does not change this or start a server.

## Format Pax source

`pax-cli fmt` formats `.pax` files and Pax templates embedded in Rust source.
It accepts a file or directory and recursively visits directories, skipping
generated and dependency folders such as `.pax`, `target`, and `node_modules`.

```sh
pax-cli fmt src
pax-cli fmt --check src
```

The first command writes formatting changes. The second checks without writing
and exits unsuccessfully if files would change, which makes it suitable for
a CI check. With no path, the command starts in the current directory.
`pax-cli format` is the full spelling; `fmt` is its alias.

Use `cargo fmt` for ordinary Rust formatting. Review formatting diffs as you
would other source changes; neither formatter verifies the interface's behavior.

## Choose the intended session

List the live sessions on your machine, then inspect the selected one:

```sh
pax-cli dev list
pax-cli dev status
```

Status reports the session ID, target platform, project location, and connection
details. Check these before using tools, particularly when several projects,
worktrees, or browser tabs are open.

Most `dev` commands accept `--path` to locate a project's active session and
`--session` to select an exact session ID. An explicit `--path` (or `-p`)
restricts selection to that project, including when the path is `.`. For
example, from a repository checkout:

```sh
pax-cli dev status --path examples/increment
pax-cli dev look --path examples/increment
```

Project selection uses the active session recorded under that project's
`.pax` directory and checks that its live registration belongs to the same
project. Relative paths are resolved from the current directory; symlink
aliases and `..` are resolved before comparing project roots. A missing or
stale active session, invalid path, or mismatched registration produces an
error. An explicit path never falls back to another project's session. Start
the intended project's debug app, or use `pax-cli dev list` to find a live ID.

`--session <id>` alone selects that exact live session regardless of the
current directory. When both `--session` and `--path` are supplied, the ID
selects the session and the path checks its project. The session need not be
the project's active session, but a different or unknown project root is
rejected before inspection, capture, or source mutation.

With **neither flag**, the CLI first tries the current directory's active
session. If none is live, it can select the sole live session anywhere on the
machine; several candidates require an explicit ID. Use `--path .` to constrain
commands to the current project, especially in automation.

This changes older CLI behavior: scripts that relied on `--path` falling back
globally must select the intended `--session` explicitly. Scripts supplying
both flags must now make them agree, or omit `--path` to select solely by ID.

Session IDs describe running sessions and can change after a restart. Re-read
them instead of hardcoding one into a permanent script. Stale registrations
are filtered from the live list.

### Which targets expose the tools?

The session-backed inspection, selector, ray-cast, and screenshot workflows
below are implemented for web and macOS debug apps. The `dev logs` command
currently supports web sessions only.

iOS and iPadOS template hot reload is a separate capability; it does not imply
that these local CLI session tools are available on a simulator or device.
Use the native app/simulator and platform debugging tools for those checks.
Release applications do not expose this development workflow.

## Inspect the running tree

Start with a shallow view:

```sh
pax-cli dev inspect tree --max-depth 3
```

The result is JSON describing the expanded application tree. It includes
instance types, parent/child relationships, layout bounds and transforms,
layer and hit-testing flags, and available source/template identifiers.
`--max-depth 0` shows only the root; omit the option when you need the full tree.

A template node can have multiple running instances, particularly inside a
loop. `engine_id` identifies a running instance; the containing component,
`template_node_id`, and source path connect it back to the template. Those
identifiers serve different purposes, and runtime IDs can change as the
interface is rebuilt. [How Pax Runs](how-pax-runs.md#from-templates-to-running-instances)
explains the expanded-tree model.

### Find an element by selector

To narrow the output to Text nodes:

```sh
pax-cli dev selector 'Text'
```

Selectors can use a type name, an ID such as `#text`, or a class such as
`.card`. Quote them so the shell passes the selector unchanged. An ID lookup
can still return multiple instances when its template is repeated. An empty
result may mean a conditional branch is not mounted or that you selected a
different session.

On the repository's Increment example, `pax-cli dev selector '#text'` finds
the count label. Its response includes both the resolved native element and
the path to `src/lib.pax`.

### Find what occupies a screen point

Capture the app at the default scale, choose a point in that image, and query
the hit stack. For a point at `(640, 360)`:

```sh
pax-cli dev ray-cast --x 640 --y 360
```

Use coordinates from the app capture, not from the whole desktop or browser
window including its toolbar. The command uses default-scale `dev look`
coordinates; avoid taking them from a resized image. It returns the nodes
under the point in z order without clicking them. `--hit-invisible` also
includes nodes normally omitted from hit testing.

This is useful when a visible element does not receive input. Check the hit
stack, the element's bounds, and anything layered above it. Earlier Pax
siblings appear in front; [Layout](layout-responsiveness.md) covers that order
and the coordinate system.

## Capture a frame or a sequence

Capture the current application:

```sh
pax-cli dev look
```

The response lists the image path, dimensions, and capture timestamp. By
default, files are written into a request-specific capture directory inside
the selected development session. You can choose an output directory:

```sh
pax-cli dev look --output-dir captures/before
```

Capture files use sequential names such as `0000.png`. Use a different output
directory for each observation you want to keep: reusing the same directory
can overwrite earlier captures.

For a five-second observation with a nominal half-second interval:

```sh
pax-cli dev look --period-ms 500 --duration-ms 5000 --output-dir captures/motion
```

Start the observation, then perform the interaction in the app. Both timing
options are required for a sequence. Capture work and application scheduling
affect when samples arrive, so use the returned timestamps and do not assume
a fixed image count. This is useful for inspecting states through an animation;
it is not a video recording or a frame-rate measurement.

`--scale 0.5` requests smaller images. PNG is the default; `--format jpeg`
selects JPEG and accepts an optional `--quality` value from `0.0` to `1.0`.
Scaling the capture does not resize the application's viewport.

Web capture combines rendered layers and native content inside the Pax mount.
An unpainted background can remain transparent. Background tabs use fallback
paths for some native text and controls, so compare the image with the live
foreground app when inspecting fine rendering or platform-widget details.
Screenshots do not replace keyboard, assistive-technology, or real-device checks.

## Read logs and diagnose a reload

For a running web session:

```sh
pax-cli dev logs --limit 50
pax-cli dev logs --follow
```

`--follow` continues until interrupted. The log reader reports browser messages
captured by the development session; compiler/build errors remain in the
terminal that runs the app. A quiet session may return no entries.

Rust logging on the web defaults to warnings in debug and errors in release.
For more detail during development, add `?pax_log=info` to the app's local URL
and reload it. If the URL already has a query, add `&pax_log=info` instead.
The supported levels are `error`, `warn`, `info`, `debug`, and `trace`.

At `info`, renderer diagnostics identify WebGPU or Vello CPU and related graphics
configuration. The same messages are available in the browser console.
Verbose output can be noisy, and captured logs are bounded recent history;
collect the relevant messages while reproducing the problem.

## Drive events and check the result

Exercise the interface through its userland input path: click or tap a control,
type into a native field, scroll its viewport, or use the keyboard. For
repeatable browser tests, a browser automation tool can perform those same
actions while Pax's inspector and captures help verify the result. Canvas
geometry often needs a coordinate-based action; native controls may expose
semantic browser targets.

**Current CLI boundary:** `pax-cli dev` does not yet provide a command to inject
a click, tap, or key event. `ray-cast` only observes hit targets. `dev touch`
changes source, as described below; it is not a touch-input driver. The runtime's
event handling remains part of Pax's open-source implementation, but a
dedicated CLI gesture interface is still a tooling gap.

Use [Event Handling](event-handling-rust.md) to understand event binding,
propagation, and handler behavior. `NodeContext::dispatch_event` delivers named
application events; it does not emulate a platform input gesture.

## Source edits through developer tools

For normal authoring, edit files in your editor and let the chosen reload
policy apply. Two `dev touch` operations also expose source changes to tools:

- `apply-component-source` replaces a component's entire `.pax` source file.
- `replace-node` replaces a template node with a Pax subtemplate and reports
  the affected source and reload scope.

Both are mutations of the project, not temporary changes to pixels. Review
the working-tree diff and target the intended session before using them.
Replacing a repeated template node can affect all of its instances. An empty
replacement subtemplate deletes the targeted node.

For example, to replace a component named `Example`, first prepare a complete
replacement file and review it. Then use `apply-component-source` with
`--component Example` and `--source-file` pointing to that file. Supply the
session ID verified by `dev status`. The command parses the source before
writing, and refuses ambiguous component-name matches; valid syntax alone
does not guarantee that the running build contains every referenced type.

Use `pax-cli dev touch --help` and the selected subcommand's `--help` for exact
arguments. A successful source write does not override a disabled reload
lane. After editing, confirm the activation and inspect the actual result.

## Read docs and example source locally

The installed CLI contains a documentation and example-source snapshot:

```sh
pax-cli docs
pax-cli docs search 'hot reload | Scroller' --limit 5
pax-cli docs open template-language
pax-cli docs examples --list
pax-cli docs examples increment
```

`docs` lists the available articles and references. Search uses `|` for
alternatives; quote the query so your shell does not treat it as a pipeline.
`open` accepts a slug, path, or title and displays the article in a terminal
pager. Example lookup prints the included source files.

These reads use the CLI's bundled snapshot, not a live fetch of the latest
website. Check `pax-cli --version` when comparing instructions across releases.
Viewing source through `docs examples` does not create a project or install
its assets. To create a runnable project from the curated bundled set, use
[`create --example`](#create-from-a-bundled-example) below.
In a repository checkout, nine curated examples live at `examples/<name>`;
focused studies, platform probes, and historical experiments live at
`examples/fixtures/<name>`. Start with the [curated example index](https://github.com/paxdotdev/pax/tree/dev/examples#readme)
or browse its fixture index. Run one from the repository root with:

```sh
cargo run --example increment       # direct native execution on macOS
cargo run -p increment              # build just this example's dependency graph
pax-cli run --path examples/increment --target web
pax-cli run --path examples/fixtures/auto-sized-containers --target web
pax-cli docs examples auto-sized-containers
```

Each example directory is an ordinary Cargo package with its own
library, native `main`, templates, and assets. The adjacent `.rs` launchers are
real Cargo example targets, including those under `fixtures`. Cargo shares the root package's dev-dependencies
across its example targets, so `-p <name>` is the narrower development command.
Moving a package into `fixtures` changes its filesystem path and CLI `--path`,
but preserves `cargo run -p <name>` and `cargo run --example <name>`.
`docs examples --list` separates curated and supporting examples; source lookup
accepts the short name or the canonical `examples/fixtures/<name>` path.
Hosted example URLs retain their short names. Starter eligibility remains
controlled by the separate bundled-example registry below.
Development profiles live at the workspace root: runtime dependencies use
optimization level 1, while each example's own code and build dependencies use
level 0. The CLI restores these profiles when exporting a standalone project.

### Web build profiles

Web builds enable WebAssembly SIMD by default. The CLI compiles `pax-runtime`,
`vello_cpu`, `vello_common`, and `fearless_simd` at optimization level 3 in both
debug and release builds so software rasterization retains its SIMD and loop
optimizations. The rest of a release cartridge remains optimized for size.
This requires a browser with WebAssembly SIMD support, including Safari 16.4
or newer. Cargo's usual flag precedence applies: if you supply `RUSTFLAGS`,
`CARGO_ENCODED_RUSTFLAGS`, or target-specific `rustflags`, include
`-Ctarget-feature=+simd128` to retain this acceleration. An explicit
`RUSTFLAGS='-Ctarget-feature=-simd128'` builds the slower scalar renderer for
browsers without SIMD; renderer selection and `@cpu`/`@gpu` work the same way.

To measure target compilation and web packaging, use an already-built CLI and
a fresh target directory without deleting your development cache:

```sh
python3 scripts/benchmark-cargo-web.py \
  --pax-cli /absolute/path/to/pax-cli \
  --target-dir /tmp/pax-build-benchmark-target \
  --logs /tmp/pax-build-benchmark-logs
```

The script builds `pax-logo` once with an empty Rust target directory and twice
unchanged, recording wall times and logs. It excludes building/installing the
CLI itself. Registry dependencies and platform tools must already be cached;
resolution is offline. Existing `.pax` output and external tool caches are
retained, so it excludes downloads and initial workstation setup. It rejects a
nonempty target directory. Select another example with `--package <name>` and
use fresh directories for each trial. The repeated warm run distinguishes
steady cache reuse from files generated during the cold build.

These measurements are not directly comparable to the earlier embedded-launcher
benchmark, which also compiled workstation tooling inside the application.
Compare compiler and packaging timings separately; a package count alone does
not establish build latency.

The CLI also has `docs build` for contributors rebuilding the documentation
assets in a Pax repository. It can regenerate API pages, example metadata/
bundles, and the CLI's search content. It is not required to browse installed
docs, and should not be run casually over an in-progress documentation edit.

The HTML docs use optimized release builds for their interactive examples.
The book's `<pax-example>` declarations select the apps and displayed source
files; the generator writes each build's `manifest.json` under
`_pax_examples/<name>/`. Short names such as `space-game` resolve to either
`examples/<name>` or `examples/fixtures/<name>`, so moving a fixture preserves its
embed URL. Generated run commands use the actual source location. Cached apps
rebuild when their source, engine/compiler inputs, Cargo manifests, or lockfile changes.
An example runs automatically while its stage is visible and suspends frame
updates and drawing when scrolled out of view or when the docs tab is hidden.
Returning to it preserves its state; **Restart** starts over, and **Open
standalone** runs the example independently. Suspension does not stop application
network requests or other external work. Elapsed-time-based animations can
advance to the current time when resumed.

### Create from a bundled example

The CLI includes complete project templates for a curated subset of examples:

| Example | Starting point |
| --- | --- |
| `pax-logo` (default) | Animated standard-library logo with a position slider, replay, and a background toggle |
| `living-quilt` | Interactive geometric tapestry with responsive components, motion, lighting, and GPU alpha masks |
| `ink-and-light` | A responsive postcard with handwriting, local assets, and Rust interaction |
| `increment` | A minimal counter for exploring the reactive update loop |

```sh
pax-cli create my-app
pax-cli create my-quilt --example=living-quilt
pax-cli create my-postcard --example=ink-and-light
pax-cli create my-counter --example=increment
```

Choose one, enter its directory, and run `pax-cli run --target web`. These
projects include the source and assets needed to build; they require no Pax
repository checkout. An unknown example name reports the available choices
without creating a partial project. The curated create set is smaller than
the source-reading catalog in `docs examples`.

Each CLI release carries its own snapshot. For contributors, `examples/<name>` and
`examples/fixtures/<name>` are canonical; `examples/bundled-cli-examples.toml` selects the create set and
`scripts/sync-cli-examples.py` synchronizes the packaged archive. Release tooling
runs that synchronization after version updates and before packaging.


## Test the Pax repository

From a monorepo checkout on macOS with the native build prerequisites installed,
run `cargo test --workspace`. No extra feature flags are required: the root
package's development dependencies align the web chassis with the designtime
runtime used by the examples. This test configuration does not change published
crate defaults or enable hot reload in release applications.

For changes to the web chassis, also run `cargo test -p pax-chassis-web --lib`
and `cargo test -p pax-chassis-web --lib --features designtime` to cover both
feature configurations. Workspace feature unification alone cannot exercise
both configurations in one invocation.

## Read more

- [Getting Started](getting-started.md) — installation and the first run.
- [Templates](template-language.md) — the source structure behind the scene.
- [Event Handling](event-handling-rust.md) — input and application logic.
- [Animation and Motion](animation-motion.md) — declarative transitions and timelines.
- [How Pax Runs](how-pax-runs.md) — runtime structure and performance reasoning.
