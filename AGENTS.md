## Background

We're working on Pax, a user interface engine focused on: 1. language ergonomics, and 2. portable high performance

Pax programs are authored in a highly declarative DSL, pax-language, which is like React, if interpolated expressions were constrained to a spreadsheet-like "formula" functional DSL. ("expressions.")

Also unlike React, Pax includes a rendering engine, a portable self-contained runtime, with a wgpu backend that renders on the GPU across web, desktop, and mobile targets -- as well as a compositing system that transforms, masks, and occludes "native elements" (like iOS form controls, or web browser UI controls) in the same coordinate space as vectors and multimedia.

The result is a highly native and high-performance medium, with a high ceiling for creative expressiveness (vectors, animations, rendering effects) as well as logical complexity (Rust-driven application logic, future support for other programming languages, declarative high-performance expressions within templates.)

## Why?

To offer a high-bandwidth, language-first way to create user interfaces for any screen, like a modern, faster, easier reimagination of HTML and CSS.

Pax is intended to give machines and humans both a more creative medium, to expand the boundaries of how humans and computers can interact.

## Pax Primer and Repo Map

Start with the local docs before inventing patterns.  The most useful entry points are:

- `pax-docs/book/src/getting-started.md` for CLI, project layout, targets, assets, and build/deploy basics.
- `pax-docs/book/src/template-language.md` for template syntax, element ordering, IDs, classes, and settings blocks.
- `pax-docs/book/src/data-binding-expressions.md` for PAXEL expressions and reactive bindings.
- `pax-docs/book/src/event-handling-rust.md` for Rust event handlers and state updates.
- `pax-docs/book/src/SUMMARY.md` for the full docs table of contents, including the API docs (generated from this codebase's code comments.)
- `examples/src/*` for code samples and working patterns.

These docs are also available by CLI: `pax-cli docs`


## Library Considerations

Because this is a foundation library and because we're focused on performance and reliability, we try to avoid certain kinds of hacks as much as possible.  Prefer correct solutions, and/or communicating with me to understand trade-offs in case a shortcut is merited.

New features are often proven on one "chassis" at a time (platform target.)  In the course of working on a chassis, if you discover a feature that seems to be missing support, do NOT hack a shortcut -- ask the user for clarification, as we may need to detour and implement a missing feature correctly.

**Binary baking maintenance callout** -- release builds now rely on baked program representations, not just the rich in-memory manifest.  Rule of thumb: if you touch manifest de/serialize behavior for anything the runtime needs in order to execute a program, audit the release cartridge path too unless the change is clearly debug-only or source-only.  More broadly, if you add or change anything that crosses the compiler/runtime boundary, expect to update the binary baking path as well.  Typical triggers include: new manifest fields that affect runtime behavior; new node kinds, control-flow semantics, property/value shapes, expression/PAXEL forms, timelines, event bindings, handler metadata, or asset references; and any change to component/type descriptors used by cartridge generation.  In practice, that usually means checking `pax-manifest`'s `program_ir`, `binary`, and `rust_manifest` paths, plus the compiler's cartridge generation/templates and the corresponding roundtrip tests.  Designtime-only/source-only metadata should usually stay out of the release-baked format unless the runtime truly needs it.

## Writing Pax

* Authoring Pax happens in two layers:

  1. `.pax` templates delcaratively describe UI trees, layout, bindings, events, animation, and style.
  2. Rust files own application state, event handlers, data loading, platform-facing logic, and any kind of side-effects.

*Reactivity* -- Pax templates are grammatically constrained to be 100% declarative.  Dynamic logic is encoded in PAXEL, an expression language that is demarcated by `{...}`.  Since the language is side-effect free, Pax is able to update properties in a "spreadsheet-like" fashion, offering many architectural and performance benefits. While powerful, this paradigm requires approaching some things differently than you might with other toolkits.

  Pax's reactive loop is:

  1. Template (Pax) declares content and structural control flow
  2. Expressions (Pax) can bind Property values or map them through logical expressions into Template settings (PAXEL, pax's spreadsheet-formula-inspired, template-embedded expression language)
  3. Event bindings (Pax) route certain interrupts (e.g. clicks) to #4
  4. Event handlers (Rust) can perform arbitrary side-effects and set Properties, which updates content and rendering reactively

  `pax_runtime_api::properties::Property` also has mechanisms for programmatic subscription and integrating into the spreadsheet like DAG that drives Pax's reactive runtime and rendering engine.

*Element order and z-order* -- in Pax, elements "on top" of others in a file are "on top" in z-index.  E.g. `<Ellipse/><Rectangle/>` will render the ellipse on top of the rectangle in z-index.  This keeps the file spatially arranged in the same way one might e.g. arrange elements in the tree view of a visual design tool, or how one might imagine looking at a stack of cards on a table.  Check z-order before debugging layout math.

*Units* -- Pax offers first-class unit support for declaring property values.  For example `25px`, `25%`, `25deg`, `25rad`. `{100% - 25px}` is an expressive construct for filling a container minus a fixed amount, and you can group units, too `(some_property + 25)px`.  Rely on % for responsive sizing.


## `pax-cli dev` AI + Developer tools

The pax-cli includes developer tools, designed for humans as well as AI use (for improving automated feedback loops.)

For example, you can take screenshots, read docs by CLI, take screenshot sequences (approx. watching a video, e.g. for an animation or interaction), trigger userland events like clicks/touches, and inspect scenes. 

Check `pax-cli dev --help` for a full rundown.

Use these tools liberally to progress your tasks with your own "eyes" and "fingertips".


## Linear Workflow

When a task starts from Linear:

1. Read the issue description, comments, status, labels, project, and linked relations before changing code.
2. Format the chat summary as "PAX-123 - Short(ened) ticket name"
3. Treat the issue text as the task contract, but resolve ambiguity against the repo's current architecture and these instructions. 
4. For any ticket of substantial scope, before commencing work, take some cycles to clarify design and requirements. Collaboration in this stage is very high leverage.
5. Reference the issue key in commit messages and PR descriptions when publishing work.
6. Leave a concise Linear update when asked to, or when the work has a meaningful blocker that is not visible from the branch/PR.
7. Please update ticket status to In Progress and In Review, to reflect current status.  The user will manage other states (such as closed.)  Do not close, cancel, or materially re-scope an issue unless the user explicitly asks.

## Git and Commit Practice

- Keep changes scoped to the task. Do not mix opportunistic refactors with feature work unless the refactor is needed to make the feature correct.
- Preserve user work. If the tree is dirty, inspect overlapping files before editing and do not revert unrelated changes.
- The user will handle git commits and operations like rebases.
- Squash commits follow a convention of "Summary commit message\n\ncommit 01's message\ncommit 02's message\n commit 03's message...
- Branches will generally be squashed before merging, to ease merge conflict management and reasoning.  Follow the squash convention above when the user requests a squash.
- Include generated files only when they are required source artifacts for this repo. Remove build output, caches, screenshots, and throwaway experiment files before final status.
- The User will occasional direct you to resolve a merge conflict.  When resolving conflicts, refer to your context for knowledge of intentional changes on the working branch.  Be sure to respect the changes made by the incoming branch, and splice logic to maintain both sets of intended changes (surfacing problematic areas for manual testing, if necessary). Don't allow work to be lost during merges, as this can be perniciously difficult to track down.

## Cleanup

Before commits, the user may ask for a cleanup pass.  Follow this protocol at those times:

- Remove vestigial experiments, commented-out code, debug logging, temporary assets, and unused imports.
- Keep tests, examples, and docs aligned with behavior changes.
- If a partial implementation must stay, make the boundary explicit with a short comment or follow-up issue reference.
- Run the narrowest meaningful validation first; broaden only when the change touches shared compiler/runtime behavior.

## Comments

- Use `///` for public Rust API documentation that should appear in generated docs.
- Use `//` for implementation notes, invariants, and short explanations of surprising code.
- Comments should explain why a choice exists, what invariant is being protected, or what external constraint is in play. Avoid repeating what the next line of code already says.
- Update or remove stale comments as part of the change that makes them stale, or when discovered in the course of adjacent work.

## Collaboration

- Whenever relevant, run a web build of the currently focused project (e.g. current example) and provide it to the user at the end of every iteration.  If we're developing for e.g. iOS or macOS per context, then run the appropriate target accordingly.
