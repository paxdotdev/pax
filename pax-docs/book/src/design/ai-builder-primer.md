# AI Builder Primer

Authoring Date: 2026-08-11

<!-- summary: Proposal for a short hosted bootstrap that helps coding agents build Pax applications through the public CLI. -->
<!-- tags: ai, agents, cli, onboarding, documentation, web -->

## Status

Proposed.

## Problem

A coding agent beginning outside the Pax monorepo has little or no Pax in its
training context. The repository-level contributor instructions are useful for
framework work, but they assume a monorepo checkout and internal source paths.
Most builders instead begin with `pax-cli`, either creating a new application
or opening an application someone else created.

The bootstrap should require almost no prompt ceremony. A builder should be
able to describe the desired result and include one memorable URL:

> Build me a spaceship shooter game with https://pax.dev/ai

The agent should infer that the URL is working context, retrieve it, and then
operate through the public Pax toolchain.

## Decision

`https://pax.dev/ai` is the canonical public address. It serves or redirects
to a static HTML response whose initial document contains the complete primer.
It must not depend on the Pax runtime or client-side routing to reveal its
substantive content.

`https://pax.dev/ai.md` exposes the same content as raw Markdown. The HTML
page links visibly to this representation and declares it as an alternate.
Humans and prompt examples use the shorter `/ai` address; `/ai.md` exists for
tools that prefer plain text.

Both representations are generated from one canonical source during the
website build so their instructions cannot drift. The web public-directory
feature can publish the resulting files as:

```text
public/ai/index.html -> /ai/
public/ai.md         -> /ai.md
```

The ordinary directory redirect makes `/ai` resolve to `/ai/`. Hosting may
serve the same content directly at `/ai` if it supports an equivalent static
rewrite, but the public prompt address does not change.

## Audience and Scope

The primer is for coding agents helping someone build a Pax application. It
assumes the Pax monorepo is not cloned and does not use internal commands such
as `cargo run -p pax-cli`, `target/debug/pax-cli`, or paths beneath
`examples/src`.

The primer may cover:

- detecting the workstation and preparing a complete worklist for missing Rust,
  platform, WebAssembly, or Pax CLI dependencies;
- installing and using the released `pax-cli`;
- creating a project or recognizing an existing Pax project;
- the two-layer `.pax` template and Rust application model;
- PAXEL as the side-effect-free expression feature of the template layer;
- finding current syntax and examples through `pax-cli docs`;
- running, inspecting, interacting with, formatting, and building an app;
- the current platform and hot-reload boundaries;
- a short set of Pax-specific mistakes that agents should actively avoid.

It is not a framework-contributor guide, exhaustive language reference, generic
prompting guide, or replacement for the documentation bundled with the CLI.

## Content Sequence

The hosted document should be short enough to read in full and opinionated
enough to change agent behavior. Its sequence is:

1. State that this is a Pax application workflow and the monorepo is not
   required.
2. Detect the workstation and existing toolchain with read-only checks. Use the
   current platform-specific Getting Started instructions to prepare one
   proposed installation and configuration worklist containing only the missing
   prerequisites, commands, expected effects, and permission-requiring steps.
3. Ask for confirmation once. After approval, execute the complete worklist as
   a batch and verify the installed versions. Return sooner only for an
   unexpected blocker or a genuinely new decision outside the approved scope.
4. If setup or a later command fails, enter the same doctor loop: inspect exact
   command output and versions, diagnose the narrowest missing or conflicting
   prerequisite, propose one consolidated repair worklist, and rerun the failing
   step after executing it. Do not continue application work while the
   toolchain is known to be broken.
5. If the project does not exist, create it with `pax-cli` and enter its
   directory. If it exists, inspect `Cargo.toml`, `src/lib.pax`, `src/lib.rs`,
   and local agent instructions first.
6. Explain the two authoring layers and the template-to-PAXEL-to-Rust reactive
   loop.
7. Route uncertain syntax to `pax-cli docs` and its embedded examples rather
   than encouraging guesses.
8. Start `pax-cli run` and treat the first successful run as the environment
   smoke check. Edit through the source files and use `pax-cli dev` to observe
   screenshots, the expanded tree, selectors, hit targets, and relevant
   interactions.
9. Explain that the debug default is `--hot-reload=pax`. Rust logic edits need
   a restart by default; web and macOS can opt into `--hot-reload=all`.
10. Require `pax-cli fmt --check` and a relevant build before handoff.

The page should prefer commands and compact invariants over long conceptual
prose. Its job is to get the agent into the live, inspectable loop quickly.

## Generated Project Instructions

The hosted primer bootstraps a session before local context exists. A generated
project then provides durable instructions at `AGENTS.md`, with `CLAUDE.md`
pointing to the same content for tools that discover that filename.

The canonical generated template lives at:

```text
pax-compiler/files/new-project/AGENTS.md
```

The compiler injects this one source into both ordinary and libdev project
templates. It should not be duplicated independently inside each template
directory. This keeps application-facing guidance versioned with the CLI that
generates it and allows commands, defaults, and supported workflows to evolve
together.

The generated file contains the durable operating contract. The hosted primer
contains enough of the same core model to bootstrap an uncreated project, but
then tells the agent to prefer the generated project's local instructions and
bundled CLI docs. Project authors remain free to extend their local file with
application-specific constraints.

## Skills and Plugins

Skills and plugins can later provide richer Pax-specific workflows, tool
bindings, or reusable interaction loops. They are complementary accelerators,
not prerequisites. The URL remains the universal bootstrap because it works
across agents without installation and can be included naturally in an ordinary
request.

## Maintenance Contract

Changes to CLI commands, default hot-reload policy, application structure, or
developer tooling should audit all three surfaces:

1. bundled `pax-cli docs`;
2. the canonical generated `AGENTS.md` template;
3. the hosted `/ai` source used to generate HTML and Markdown.

The hosted page should display a last-updated date and the Pax CLI release it
was validated against. It should avoid pinning install commands to a version
unless compatibility requires it.
