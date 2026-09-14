<a id="api-docs"></a>

# API Reference
<!-- summary: Find exact Pax properties, types, events, and methods, with links back to their authoring guides. -->
<!-- tags: api, reference, properties, components -->
<!-- Generated into book/src/api/index.md by gen_api_docs; edit this template. -->

Use the API reference to look up a type, inspect a component's properties,
or check a method signature while building. The pages are generated from Rust
declarations and documentation comments in the Pax source tree. The guide
chapters explain the concepts and show how to combine these APIs into an
interface.

If you are starting a project, begin with [Getting Started](../getting-started.md).
For an existing project, match the documentation version to your Pax dependency.
The development reference may contain changes that are absent from your release;
a newer reference page does not establish support in an older build.

## Find an API

| What you are working on | Reference | Guide |
| --- | --- | --- |
| Reactive state and computed values | [Properties](pax-runtime-api/properties.md) | [State and Properties](../state-properties.md) |
| Input events and handler arguments | [Events](pax-runtime-api/events.md) | [Events and Rust](../event-handling-rust.md) |
| Text and fonts | [Text](pax-std/core/text.md) | [Text, Fonts, and Images](../text-fonts-images.md) |
| Shapes, paths, and paint | [Drawing values](pax-runtime-api/drawing.md) and [drawing elements](pax-std/drawing.md) | [Drawing and Styling](../drawing-styling.md) |
| Routes and navigation | [Router](pax-std/core/router.md) | [Routing](../routing.md) |

## Read a reference page

A type's Rust declaration shows its field and method types. For a Pax element,
the guide supplies the corresponding template usage: a Rust field such as
`Property<String>` is an input you can set with a compatible literal or binding.
[Templates](../template-language.md#setting-values) and
[PAXEL](../data-binding-expressions.md#bindings) explain that authoring syntax.

Read the documentation alongside the signature for defaults, behavior, and
target restrictions. An item appearing in the reference does not establish
that every backend or platform implements it equally. The relevant guide
and source for your release provide the context; the
[targets chapter](../targets-build-deploy.md) explains the build boundary.

## Public crates

`pax-runtime-api` contains author-facing values and reactive APIs. `pax-std`
contains the standard elements and components used in templates. Browse their
modules when you need more detail than the entry points above:
