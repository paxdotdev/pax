# Roadmap
*As of Q2 2024*

This document contains an unordered list of currently anticipated work for Pax Engine.  This is intended both as a guide for folks keeping an eye on where Pax is headed, as well as a starting point for folks interested in contributing to the open source project.

### Public release of Pax Create
So much of the value of using Pax comes from being able to create visually as well as with code.  We are working to release a public beta of Pax Create as soon as possible.

### Double binding
Currently Pax supports only one-way reactive bindings, from parent to child component.  To send data back the other direction requires using a channel mechanism and/or static data.  While this is viable, it is inelegant, and particularly for the common cases where you want to double-bind a single piece of state, this is something we should support out-of-the-box.  Syntactically this might look like `<Foo bind:some_child_property=some_self_property />` (two-way) instead of `<Foo some_child_property=some_self_property />` (default, one-way), where the Pax runtime will treat those two properties as pointers to the same singleton, thus a change in one is a change to the other.

### GPU rendering
Currently Pax renders using platform-native CPU canvas APIs.  Pax’s rendering layer is quite thin — we have even achieved GPU rendering with WGPU [https://github.com/paxengine/pax/pull/76], however the wasm footprint for the WebGL backend was too heavy (10MB+, vs. our target of 100KB).  In addition to wgpu, we are interested in scoping out rive-renderer, however the need to statically link their runtime as well as their renderer seems like it will also be too heavy for our footprint goals.  Likely WGPU + tessellation (Lyon) + caching meshes through our reactive system will be the way forward, while either stomaching the WebGL footprint overhead, or contributing necessary changes to wgpu to fix the footprint.

### TypeScript bindings
Write Pax with TypeScript instead of Rust

### Standard library:
- Dropdown list
- Slider component
- More controls & styling options for existing standard library components like `<Button />`
- File upload / open component
- File download / save component
- Various visual flair components, ambient decorations, and useful / visually interesting effects.

### Rendering features:
- Masking
- Non-rectilinear clipping
- Occlusion optimization: currently new canvas layers are created fairly naively, impacting CPU and memory performance.  We can be more intelligent about creating new occlusion layers, namely only when native elements vs. vector elements actually occlude each other.

### Static analysis:
Currently Pax uses `dynamic analysis` to reflect on the property + struct schema of a Rust project, running a variant of your program with the `parser` feature enabled to perform reflection and aggregate results.
This dynamic analysis introduces substantial complexity to our build process, and all of this work could be done strictly statically, similarly to how `rust-analyzer` crawls a program to find property + struct schema.

### Ternaries in PAXEL:
Using ternaries in expressions is a powerful way to compress logic — currently to express conditions in PAXEL you must use an outer if statement + two mostly duplicated arms of template: `if foo == bar { <SomeComponent prop=a /> } if foo != bar {<SomeComponent prop=b />}.  This can be simplified with PAXEL ternaries like `<SomeComponent prop={foo == bar ? a : b} />`.  As a syntactic alternative, since PAXEL compiles into Rust we could easily rely on Rust's treatment of if statments as expressions instead of the more arcane ternary syntax, e.g. `<SomeComponent prop={if foo == bar { a } else { b }} />`

### Router component
Similar to e.g. React Router, but designed with cross-platform concerns in mind.  

### Else statements in templates:
Pax includes control flow in templates, namely `for` and `if`.  Currently `else` is not supported on `if` statements, requiring the authoring of manually complementary if statements `if foo {<VariantA />} if !foo {<VariantB />}`.  This work extends to `else if`, as well.

### Error messages++:
There are multiple classes of error messages in Pax, including compile-time errors (Rust,) parse-time errors (Pax or PAXEL), runtime errors (Rust) and runtime errors (PAXEL evaluation or Pax runtime).
We have basic support for each of these classes of error, however ongoing work is required to make these error messages more robust and helpful.  Some simplification here will be unlocked with the `static analysis` refactor to the Pax compiler.

### PAXEL interpreter:
Currently PAXEL is transpiled into Rust and built statically into machine code.  This is fast, but it requires full recompilation when oftentimes during authoring, it is preferable to interpret in order to support live updates + edits.  The PAXEL interpreter will be particularly useful when using Pax Create, where expression changes currently require painful cycles with the Rust compiler.

### Grammar improvements & shorthands:
Various aspects of the Pax grammar can be polished and improved for certain APIs.  For example, `<Rectangle />` border radius currently requires a verbose Rust enum literal via PAXEL like `<Rectangle corner_radii={RectangleCornerRadii::radii(7.0, 7.0, 7.0, 7.0) /> — however, this could be simplified to `<Rectangle corner_radii=7 />` (homogeneous shorthand) or `<Rectangle corner_radii=(7,7,7,7) />` (heterogeneous tuple shorthand).  Text APIs and gradient APIs are other prime candidates for grammatical polish.

### Universal embedded component wrappers:
Pax is designed to run anywhere, including inside existing codebases.  To make this easier to execute, we intend to build component wrappers that make Pax “just work” e.g. with React, Vue, NextJS, Angular, and WebComponents, as well as SwiftUI Views and other cross-platform wrappers.

### Pax Doctor:
Especially when building across platforms, various toolschains and dependencies are required on your workstation. Though we document these as thoroughly as possible in our docs, it would be convenient if the CLI could tell you automatically what is missing, and even install missing dependencies automatically.  Prior art includes Flutter doctor.

### Pax Playground:
It should be possible to play with Pax without installing any toolchains at all, through an online, turnkey playground, similar to the Rust playground.

### Cross-platform targets:

#### (1) Mobile:
**Productionize iOS target:**
While Pax currently compiles and runs as native iOS apps, there is work needed to make compilation and syndication, e.g. publishing to the iOS App Store, seamless.  There’s also work needed to better integrate with iOS developers’ workflows, e.g. a SwiftUI View wrapper.

**Build Android target:**
Choose which layer and toolkit to build with (Kotlin vs. Java, Jetpack Compose vs. other) and build robust support

#### (2) Desktop:
**Productionize macOS target:**
Pax currently builds to native macOS apps, however work is required to make this build target robust and production-ready.  The primary value in this target currently is for LLVM-specific tooling, e.g. debuggers, which aren’t yet able to debug wasm builds.

**Build Windows + Linux targets:**
As “natively” as possible (which has some meaning in Windows, and roughly no meaning in Linux,) build support for Windows and Linux deployment targets.  For Windows this work includes integrating with the best fit “native” Windows UI library (the space is fragmented,) and for Linux it means building a 100% vector-rendered toolkit for the standard library, including text and form controls.  Existing libraries like Iced or CosmicUI may be portable here.
