## Background

We're working on Pax, a user interface engine focused on: 1. language ergonomics, and 2. portable high performance

Pax programs are authored in a highly declarative DSL, pax-lang, which is like React, if interpolated expressions were constrained to a spreadsheet-like "formula" functional DSL. ("expressions.")

Also unlike React, Pax includes a rendering engine, a portable self-contained runtime, with a wgpu backend that renders on the GPU across web, desktop, and mobile targets -- as well as a compositing system that transforms, masks, and occludes "native elements" (like iOS form controls, or web browser UI controls) in the same coordinate space as vectors and multimedia.

The result is a highly native and high-performance medium, with a high ceiling for creative expressiveness (vectors, animations, rendering effects) as well as logical complexity (Rust-driven application logic, future support for other programming languages, declarative high-performance expressions within templates.)

## Why?

To offer a high-bandwidth, language-first way to create user interfaces for any screen, like a modern, faster, easier reimagination of HTML and CSS.

Pax is intended to give machines and humans both a more creative medium, to expand the boundaries of how humans and computers can interact.

## Library Considerations

Because this is a foundation library and because we're focused on performance, we try to avoid certain kinds of hacks as much as possible.  Prefer correct solutions, and/or communicating with me to understand trade-offs in case a shortcut is merited.

New features are often proven on one "chassis" at a time (platform target.)  In the course of working on a chassis, if you discover a feature that seems to be missing support, do NOT hack a shortcut -- ask the user for clarification, as we may need to detour and implement a missing feature correctly.

## Considerations for writing Pax

While Pax is designed to feel familiar and "HTML-like," there are some design decisions that may counter some expectations from working with other technologies.

*Element order and z order* -- in Pax, elements "on top" of others in a file are "on top" in z-index.  E.g. `<Ellipse/><Rectangle/>` will render the ellipse on top of the rectangle in z-index.  This keeps the file spatially arranged in the same way one might e.g. arrange elements in the tree view of a visual design tool, or how one might imagine looking at a stack of cards on a table.

*Units* -- Pax offers first-class unit support for declaring property values.  For example `25px`, `25%`, `25deg`, `25rad`, and even expression constructs like `50% + 25px` or `(some_value + 25)%` are valid.

*Constrained reactivity* -- Pax templates are grammatically constrained to be 100% declarative.  Dynamic logic is encoded in PAXEL, an expression language that is demarcated by `{...}`.  Since the language is side-effect free, Pax is able to update properties in a "spreadsheet-like" fashion, offering many architectural and performance benefits. While powerful, this paradigm requires approaching some things differently than you might with other toolkits.

## AI + Developer tools

The pax-cli includes developer tools, designed for AI use, for improving automated feedback loops.

For example, you can take screenshots, take screenshot sequences (approx. watching a video, e.g. for an animation or interaction), trigger userland events like clicks/touches, and inspect scenes. 

Check `pax-cli dev --help` for a full rundown.

Use these tools liberally to progress your tasks with your own "eyes" and "fingertips".