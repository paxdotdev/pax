# Build with Pax

Pax combines HTML-like markup with spreadsheet-like expressions and Rust
application logic. Use this guide with Codex, Claude Code, or another coding
agent to turn an idea into a running Pax application.

Start with a small, working web app. Read the documentation shipped with the
installed CLI, use real Pax components, and inspect the result in a browser.
This guide is also available as [raw Markdown](/ai.md).

Work through the released `pax-cli` and the application project. Do not assume
the Pax framework monorepo is cloned, and do not clone it unless the user is
explicitly asking to change Pax itself.

The user's request remains the objective. This document supplies the Pax
workflow and technical constraints needed to accomplish it.

## Establish a Working Environment

Before application work, determine whether the workstation can build and run a
Pax web project. Start with read-only checks:

- identify the operating system and architecture;
- inspect `rustc`, `cargo`, `rustup`, `wasm-pack`, and `pax-cli` availability and
  versions;
- verify that the `wasm32-unknown-unknown` Rust target is installed;
- on macOS, inspect the Xcode command-line tools when an Apple target is needed;
- on Linux or Windows, inspect the native build prerequisites described by the
  current Pax Getting Started guide.

Use the current platform instructions at
<https://docs.pax.dev/getting-started.html>. Do not guess at substitute packages or
unsupported shortcuts when an official setup path exists.

If anything is missing or misconfigured, act as a setup doctor:

1. Gather the relevant command output and version information without changing
   the workstation.
2. Prepare one consolidated, platform-specific installation and configuration
   worklist. Include only missing prerequisites, list the exact commands and
   expected effects, and identify any elevated permissions or external tools.
3. Present the worklist and obtain any approval required by the user and the
   agent's permission settings. Do not bypass those settings.
4. Execute the approved steps. Return to the user for an unexpected blocker or
   a genuinely new decision not covered by the approval.
5. Verify the resulting versions and rerun the command that exposed the problem.

Batch setup decisions where the agent's permissions allow it. Verify the
toolchain before proceeding with application work.

The usual web setup includes Rust, the `wasm32-unknown-unknown` target,
`wasm-pack`, and the Pax CLI:

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-pack --version 0.15.0
cargo install pax-cli
```

These commands are not a complete substitute for the platform-specific guide.
For example, Windows and Linux require native build tooling, and Apple targets
require macOS and Xcode.

## Create or Open the Application

If the user has an existing Pax project, enter its root and inspect these files
before editing:

- `AGENTS.md` or equivalent local agent instructions;
- `Cargo.toml`;
- `src/lib.pax`;
- `src/lib.rs`;
- existing components, assets, and project-specific documentation.

Local project instructions take precedence over this general primer.

If no project exists, create one with the released CLI:

```sh
pax-cli create my-app
cd my-app
pax-cli run --target web
```

Choose an appropriate project name from the user's request instead of using
`my-app` literally. `pax-cli run` targets the web by default. Treat the first
successful run as the environment smoke check before building substantial
features.

## Understand the Two Authoring Layers

Pax applications have two layers:

1. `.pax` templates declaratively describe the interface tree, layout,
   bindings, events, animation, and style. PAXEL expressions inside `{...}` are
   a side-effect-free feature of this template layer.
2. Rust owns application state, event handlers, data loading, platform-facing
   logic, and other side effects.

The normal reactive loop is:

1. A template declares content and structure.
2. PAXEL derives displayed and rendered values from reactive properties.
3. Template event bindings route user input to Rust handlers.
4. Rust handlers perform effects and update properties.
5. Pax propagates the changed dependencies back through layout and rendering.

Do not put side effects into PAXEL or invent imperative template behavior. Use
Rust handlers when work must mutate state or interact with the outside world.

## Find Current Syntax Before Guessing

Pax is pre-1.0 and has limited external training data. Use the documentation
and examples bundled with the installed CLI as the source of truth:

```sh
pax-cli docs
pax-cli docs open getting-started
pax-cli docs open template-language
pax-cli docs open data-binding-expressions
pax-cli docs open event-handling-rust
pax-cli docs search <query>
pax-cli docs examples --list
pax-cli docs examples <example-name>
```

Prefer adapting a current working example over extrapolating unfamiliar syntax
from another GUI framework. The installed CLI's docs and examples are a
version-matched snapshot; online docs may describe a newer release. Check
`pax-cli --version` and command `--help` when instructions differ. Do not silently
switch to an unreleased framework checkout to obtain a missing feature.

Important Pax-specific rules include:

- Source order defines stacking order. An earlier sibling is drawn above a
  later sibling.
- Units are first-class values, including `px`, `%`, `deg`, and `rad`.
  Expressions such as `{100% - 25px}` are valid and useful for responsive
  layout.
- Percentage positions also imply a percentage anchor: `x=50%` centers an
  element. Set `anchor_x=0%` explicitly to position its left edge halfway across.
- Use percentage sizing where appropriate so the interface responds across
  window and device sizes.
- Keep application media in `assets/`. Use `public/` only for web responses that
  must exist independently of the running Pax application.

## Use the Live, Inspectable Loop

Keep `pax-cli run` active in a dedicated terminal while iterating. The debug
default is `--hot-reload=pax`: `.pax` edits update the mounted application,
while Rust edits require rebuilding and restarting. Reload the browser after
rebuilding a web app. Inline templates in `.rs` files also need a rebuild.

```sh
pax-cli run --target web --hot-reload=pax
```

The supported reload modes are `pax` and `off`. `all` is a legacy alias for
`pax`; `logic` is retired. Release builds disable hot reload. See
[Developer Workflow & Tools](https://docs.pax.dev/developer-workflow.html#hot-reloading).

Use the developer tools to observe the application rather than inferring its
behavior from source alone:

```sh
pax-cli dev look
pax-cli dev inspect tree
pax-cli dev selector --help
pax-cli dev ray-cast --help
pax-cli dev touch --help
pax-cli dev logs --help
```

Capture screenshots, inspect the expanded scene tree, exercise relevant
interactions, and use hit-testing or logs when selection and event routing are
unclear. If the request is visual or creative, take a deliberate design pass
and iterate on the rendered result until it is coherent, responsive, and
faithful to the requested direction.

## Current Target Boundaries

Start with the web target unless the user requests another platform. The web
install/create/run workflow supports macOS, Debian/Ubuntu Linux, and Windows.
Native macOS, iOS, and iPadOS builds require macOS and full Xcode.

Android is experimental; native Windows and Linux hosts are alpha, with
narrower control support and specific GPU requirements. Check the installed
version and the [target guide](https://docs.pax.dev/targets-build-deploy.html)
before promising a capability. GPU and CPU rendering do not have feature
parity: features such as 3D and lighting require a GPU-backed renderer.
Rust is the application-logic language; do not promise JavaScript or Python
application authoring.

## Verify Before Handoff

Complete the work rather than stopping after the first successful compile:

1. Exercise the requested behavior in a running application.
2. Inspect the rendered result, including relevant responsive sizes and input
   paths.
3. Format and verify Pax sources.
4. Complete at least a web build unless the user's requested target requires a
   different or additional build.
5. Report what was verified and any remaining target-specific limitations.

```sh
pax-cli fmt
pax-cli fmt --check
pax-cli build --target=web
```
