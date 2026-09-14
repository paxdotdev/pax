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

Debug runs use **Pax-only hot reload** by default. Saving a `.pax` template
can update the mounted interface. Changing Rust application logic requires
stopping and rerunning the application unless you opt into logic reload.

The two reload lanes are independent:

| Mode | Saved `.pax` changes | Saved application-logic changes |
| --- | --- | --- |
| `pax` — default | Reload | Rebuild and restart manually |
| `all` | Reload | Rebuild and activate automatically on supported targets |
| `logic` | Wait for the next logic rebuild or restart | Rebuild and activate automatically on supported targets |
| `off` | Wait for restart | Rebuild and restart manually |

Choose a mode for a run:

```sh
pax-cli run --target web --hot-reload=pax
pax-cli run --target web --hot-reload=all
pax-cli run --target web --hot-reload=logic
pax-cli run --target web --hot-reload=off
```

Run one of these at a time. `all` is useful when iterating between a template
and its Rust handlers. A logic reload still compiles application code; allow
the build and activation to finish before judging the changed behavior.

### Target support and restart boundaries

Web and macOS support both lanes. iOS and iPadOS support `.pax` reload, while
Rust logic changes require rebuilding and relaunching. On those mobile
targets, `all` keeps template reload enabled and reports that saved logic
changes need a restart. `logic` alone is rejected as unsupported.

Some template edits also need new compiled Rust information—for example, a
new component type or handler that the running build does not know about.
Check the reload diagnostics and rebuild when necessary. Inline templates
inside `.rs` files follow the Rust-source change path.

Disabling a lane leaves your saved files intact. Their changes become part of
the next applicable rebuild or restart. With `off`, the development service
continues to support inspection; it simply stops activating source edits.

Reloading can replace component instances or the application. Do not rely on
arbitrary local state, focus, or scroll position surviving every kind of
reload. Reproduce important test state explicitly. A failed logic build keeps
the previously active revision available; read the error, correct the source,
and wait for a successful activation.

Release builds disable both lanes. Use a debug session for editing and tools,
then check the actual release output before shipping. [How Pax Runs](how-pax-runs.md#debug-and-release)
explains that boundary.

### Configure a project default

To store the preference with the project, add this to `Cargo.toml`:

```toml
[package.metadata.pax.dev]
hot_reload = "all"
```

The accepted values are `pax`, `all`, `logic`, and `off`. Selection precedence
is the `--hot-reload` flag, then the `PAX_HOT_RELOAD` environment variable,
then Cargo metadata, then the `pax` debug default. An explicit flag is useful
when checking whether a shell or project preference is affecting a run.

Generated `cargo run` wrappers forward trailing arguments, so
`cargo run -- --hot-reload=all` can select the same policy.

<div class="docs-example-placeholder">
<p><strong>Workflow demonstration planned:</strong> one template edit followed by one Rust-handler edit, with the source, running app, and reload status visible together.</p>
<!-- Production brief:
- Use one canonical example and name its source revision. Show the default Pax
  lane first, then the optional all mode; include a real failure/recovery step.
- Three treatments: an annotated terminal/app recording; a four-step filmstrip;
  or a source diff beside before/after captures. Prefer the filmstrip with a
  short optional recording. Do not imply a fixed reload duration.
- Keep the demonstration separate from the PAX-993 starter's evolving design. -->
</div>

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
`--session` to select an exact session ID. The explicit ID takes precedence.
For example, from a repository checkout:

```sh
pax-cli dev status --path examples/src/increment
```

Project selection normally finds the active session recorded under that
project's `.pax` directory. If it cannot, the CLI falls back to the single live
session on the machine; with several candidates, it asks for an explicit ID.
Consequently, `--path` alone is not an isolation guarantee. Before an automated
source edit, check status and use the intended session ID.

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

At `info`, renderer diagnostics identify WebGPU or Piet and related graphics
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
its assets. In a repository checkout, canonical examples live under
`examples/src`; run one from the repository root with:

```sh
pax-cli run --path examples/src/increment --target web
```

The CLI also has `docs build` for contributors rebuilding the documentation
assets in a Pax repository. It can regenerate API pages, example metadata/
bundles, and the CLI's search content. It is not required to browse installed
docs, and should not be run casually over an in-progress documentation edit.


## Read more

- [Getting Started](getting-started.md) — installation and the first run.
- [Templates](template-language.md) — the source structure behind the scene.
- [Event Handling](event-handling-rust.md) — input and application logic.
- [Animation and Motion](animation-motion.md) — declarative transitions and timelines.
- [How Pax Runs](how-pax-runs.md) — runtime structure and performance reasoning.
