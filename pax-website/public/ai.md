# Build with Pax

Pax is a language-first GUI framework for Rust. Use this guide when helping
someone create or modify a Pax application.

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
<https://docs.pax.dev/getting-started/>. Do not guess at substitute packages or
unsupported shortcuts when an official setup path exists.

If anything is missing or misconfigured, act as a setup doctor:

1. Gather the relevant command output and version information without changing
   the workstation.
2. Prepare one consolidated, platform-specific installation and configuration
   worklist. Include only missing prerequisites, list the exact commands and
   expected effects, and identify any elevated permissions or external tools.
3. Present that single worklist and ask for confirmation once.
4. After confirmation, execute the approved work as one batch. Return to the
   user sooner only for an unexpected blocker or a genuinely new decision that
   was not covered by the approval.
5. Verify the resulting versions and rerun the command that exposed the problem.

Do not make the user approve each ordinary installer step separately. Do not
continue application work while the toolchain is known to be broken.

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
pax-cli run
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
pax-cli docs list
pax-cli docs open "Getting Started"
pax-cli docs open "Template Language & Structure"
pax-cli docs open "Data Binding & Expressions"
pax-cli docs open "Event Handling & Rust Logic"
pax-cli docs search <query>
pax-cli docs examples --list
pax-cli docs examples <example-name>
```

Prefer adapting a current working example over extrapolating unfamiliar syntax
from another GUI framework.

Important Pax-specific rules include:

- Source order defines stacking order. An earlier sibling is drawn above a
  later sibling.
- Units are first-class values, including `px`, `%`, `deg`, and `rad`.
  Expressions such as `{100% - 25px}` are valid and useful for responsive
  layout.
- Use percentage sizing where appropriate so the interface responds across
  window and device sizes.
- Keep application media in `assets/`. Use `public/` only for web responses that
  must exist independently of the running Pax application.

## Use the Live, Inspectable Loop

Keep `pax-cli run` active in a dedicated terminal while iterating. The debug
default is `--hot-reload=pax`: `.pax` edits update the mounted application,
while Rust edits require rebuilding and restarting by default.

On web and macOS, opt into application-logic reload when its build latency is
appropriate:

```sh
pax-cli run --hot-reload=all
```

iOS and iPadOS require rebuilding and relaunching after Rust logic changes.

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

Pax applications currently target web browsers through WebAssembly, macOS,
iOS, and iPadOS. The web install, create, and run workflow is supported on
macOS, Debian/Ubuntu Linux, and Windows. Apple application targets require a
supported macOS workstation and Xcode.

Do not present planned Linux, Windows, Android, JavaScript, or Python application
support as shipping today.

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
