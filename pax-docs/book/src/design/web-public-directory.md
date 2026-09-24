# Web Public Directory

Authoring Date: 2026-08-11

<!-- summary: Specification for publishing root-relative static files in Pax web builds and development sessions. -->
<!-- tags: web, build, static-files, cli, routing -->

## Status

Accepted for implementation.

## Problem

Pax web applications sometimes need HTTP responses that are available before
the client runtime loads. Examples include Markdown or text primers, robots
directives, well-known metadata, and manually authored HTML. These are not
application UI routes: a browser, crawler, or command-line client must be able
to retrieve their substantive bytes without initializing the Pax runtime.

Pax currently copies application assets beneath `/assets` and serves missing
HTML routes through an `index.html` history fallback. It does not give ordinary
application authors a way to publish files at arbitrary root-relative paths.
The advanced `interfaces/web/public` hook replaces the generated web interface
and is too broad for this job.

## Goals

- Publish ordinary files at predictable root-relative web paths.
- Use a visible filesystem convention rather than Cargo metadata.
- Keep static HTTP responses separate from the cross-platform runtime Router.
- Produce a self-contained deployable directory from `pax-cli build`.
- Let `pax-cli run` serve source files directly so they can be edited without a
  rebuild or restart.
- Prevent public files from replacing generated runtime artifacts or escaping
  the project through symbolic links.

## Non-Goals

- Static rendering or server-side rendering of Pax components.
- Redirects, aliases, response headers, or cache-policy configuration.
- Markdown-to-HTML generation or other content transforms.
- Packaging the public tree into Apple applications.
- Replacing the existing custom web-interface hook.
- Automatically refreshing an open browser when a public file changes.

## Authoring Model

A project may contain a `public/` directory beside its `Cargo.toml`. Each file
maps to the same relative path at the web root:

```text
public/ai.md             -> /ai.md
public/robots.txt        -> /robots.txt
public/.well-known/pax   -> /.well-known/pax
public/ai/index.html     -> /ai/
```

The directory is optional. Pax does not generate an empty `public/` directory
for new projects.

Files are copied byte-for-byte. Nested directories and dotfiles are preserved.
Pax performs no hashing, minification, Markdown rendering, or other content
transformation.

## Build Behavior

The feature applies only to web targets. The project root is the directory that
contains the selected `Cargo.toml`, rather than the process working directory.

For `pax-cli build`, Pax first materializes the generated or custom interface,
project metadata such as a favicon, the Wasm cartridge, and application assets.
It then validates the complete public tree and copies it into the web output
before producing the final build directory. Debug, release, and profiling
builds use the same behavior.

Validation completes before the first public file is copied. A collision must
therefore fail without partially overlaying generated output.

## Development Serving

For `pax-cli run`, the generated build output does not contain a snapshot of
the public tree. The development server instead composes two filesystem roots:

1. Generated output for the application shell, cartridge, and application
   assets.
2. The project's source `public/` directory.

Requests resolve in this order:

1. exact generated file;
2. exact public file;
3. the client-side `index.html` history fallback when the request accepts HTML
   and does not target a file-like path;
4. 404.

Generated files take precedence so a public file cannot shadow the running
application. Public files still take precedence over the history fallback.
The server reads the source file on each request, so edits, additions, and
deletions are visible after a refresh or refetch without rebuilding Pax.

Directory requests may use `index.html` and redirect to a slash-terminated
path. Hidden path segments are allowed for conventions such as `.well-known`.
The development server infers content types from file extensions and signals a
UTF-8 charset for text content.

Automatic browser refresh is separate from static serving. The current project
watcher notifies designtime clients about non-source changes but does not expose
a browser live-reload channel. Manual refresh is sufficient for this first
implementation.

## Collision and Safety Rules

An exact public file cannot replace an existing generated or custom-interface
file. A file-versus-directory conflict also fails. Public directories may merge
with existing directories only when none of their contained files conflict.

The following top-level directories are reserved even when a particular build
has not materialized them yet:

- `assets`
- `snippets`
- `__reloads__`

Other generated names, including `index.html`, cartridge artifacts, interface
scripts, package metadata, and generated favicons, are protected by exact
collision checks against the completed interface.

The public directory and everything beneath it must be ordinary directories or
regular files. Symbolic links and other special filesystem entries are rejected
during builds. Development requests also reject paths containing symbolic-link
components so a live server cannot expose a file outside the project.

## Relationship to Assets and Router

`assets/` remains the source of application media loaded by Pax across web and
Apple targets. Those files are copied beneath `/assets` and participate in the
existing compiler/runtime asset model.

`public/` is a web packaging and serving surface for responses that exist
independently of the mounted application. It is intentionally not represented
by `Router`, because Router is a cross-platform runtime control-flow primitive
that runs only after the application has loaded.

## Deferred Extensions

The 22 September routing decision adds validated
`[package.metadata.pax.web].server_owned_prefixes` for paths where Pax yields to
ordinary browser navigation. It also excludes missing files beneath those paths
from the local server's application fallback. This uses the existing Cargo web
namespace; it supersedes this proposal's earlier recommendation against Cargo
metadata for future serving configuration. Production origin routing, redirects,
response headers, and cache policies still belong to the hosting configuration.
