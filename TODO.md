# TODO


## Milestone: proof of concept

[x] Rendering 
[x] Components 
[x] Logging
[x] Stroke, color, fill
[x] Sizing
    [x] Browser resize support
    [x] None-sizing
    [x] Transform.align
    [x] Transform.origin
[x] Expression engine
    [x] variables, declaration & storage
    [x] node IDs
    [x] summonables
    [x] built-in vars like frame count
    [x] MVP rust closures + manifest of deps
[x] Spreads (née Stacks)
    [x] Decide `primitive` vs. userland `components`
    `components`
    [x] Internal template mechanism for components
    [x] Make `root` into a component definition
    [x] Control-flow `placeholder` (`placeholder`) for inputs/children
    [x] Ensure path forward to userland `placeholders`
    [x] Clipping & Frames
    [x] Control-flow `repeat` for cells & dividers inside template
    [x] Gutter
[x] Split out userland code
    [x] Add a third project to workspace, the sample project
    [x] (Further work to be done via compiler task)
[x] Timelines, transitions
[x] Refactors
    [x] Bundle Transform into "sugary transform," incl. origin & align; consider a separate transform_matrix property
    [x] Is there a way to better-DRY the shared logic across render-nodes?
e.g. check out the `get_size` methods for Frame and Spread
    [x] Maybe related to above:  can we DRY the default properties for a render node?
Perhaps a macro is the answer?
    Same with `scale`
    [x] Can we do something better than `(Box<......>, Box<.......>)` for `Size`?
    [x] Rename various properties, e.g. bounding_dimens => bounds
    [x] Take a pass on references/ownership in render_render_tree — perhaps &Affine should transfer ownership instead, for example
    [x] Better ergonomics for `wrap_render_node_ptr_into_list`
    [x] Evaluate whether to refactor the `unsafe` + PolymorphicType/PolymorphicData approach in expressions + scope data storage



## Milestone: "hello world" from .pax


[x] Compile base cartridge
    [x] Refactor PropertiesCoproduct to its own module
    [x] Sanity check "patch" ability for "blanks" (Properties, Expressions)
    [x] Demo app chassis running example project (`./serve.sh`)
        [x] Add stub macro for `pax`, derives
[ ] `pax-compiler`
    [x] architecture, seq. diagram
    [ ] two-stage compilation process
        [x] thread/process/IPC chassis
        [x] parser cargo feature
        [x] bin-running harness to execute parser (see https://stackoverflow.com/questions/62180215/renaming-main-rs-and-using-with-cargo)
        [ ] TCP message passing
            [ ] de/serialization for manifest
            [ ] coordination of TCP components from compiler main thread
        [ ] parse and load .pax files
            [x] load file via macro
            [ ] generate the necessary bits via macro
            [ ] port minimal set of std entities (Rectangle, Group) to support manifest-gen 
            [ ] traverse manifest of Component defs: parse .pax files, store in mem
            [x] (start with templates only)
    [x] thread for wrapping `cargo build`
    [x] sketch out .pax folder design
    [ ] codegen PropertiesCoproduct
    [ ] codegen DefinitionToInstance traverser (at least `match` block)
    [ ] codegen Cargo.toml + solution for patching
        [ ] Maybe need to "deep patch" pax-properties-coproduct within core, dep. on how Cargo resolves `patch`
    [ ] graceful shutdown for threaded chassis (at least: ctrl+c and error handling)
    [ ] maybe codegen RIL INSTEAD OF designtime coordination;
        as a path to quicker PoC
[ ] `pax-message`
    [ ] design structs (central enum in own package?) for representing messages
    [ ] de/serialization of messages with Serde
[ ] macros
    [ ] write manual expanded form
    [ ] write automated expanded form
    [ ] mechanism to ensure every macro is invoked each compilation (or otherwise deterministically cached)
    [ ] Phone home to macro communication server to register Components/Properties   
        - component names/paths and pax file paths 
        - property schemas assoc. with components
[ ] designtime
    [ ] legwork for `Definitions` and `Instances`
        [ ] Write `Definition` structs and refactor existing entities to `Instance` structs
        [ ] Write ORM methods for `Definitions`
        [ ] Attach CRUD API endpoints to `Definition` ORM methods via `designtime` server
    [ ] figure out recompilation loop or hot-reloading of Properties and Expressions


[ ] baseline primitive(s) for hello world
    [ ] import/package management
    [ ] RIL -> PAX compatibility, or rewrite primitives
[ ] render Hello World
    [ ] Manage mounting of Engine and e2e 

## Milestone: clickable square

[ ] Action API
    [ ] state management (.get/.set/etc.)
    [ ] Instantiation, reference management, enum ID + addressing for method definitions &
        invocations
    [ ] tween/dynamic timeline API
[ ] Event capture and transmission
    [ ] Map inputs through chassis, native events (mouse, touch)
        [ ] PoC with Web
    [ ] Message queue in runtime
    [ ] Ray-casting? probably
    [ ] Message bubbling/capture or similar solution

> What's our expression language MVP?
> - `==`, `&&`, and `||`
> - Parenthetical grouping `(.*)`
> - Literals for strings, bools, ints, floats
> - Nested object references + injected context

[ ] Expressions
    [ ] Transpile expressions to Rust (or choose another compilation strategy)
    [ ] Write ExpressionTable harness, incl. mechanisms for:
        [ ] vtable storage & lookup
        [ ] Dependency tracking & dirty-watching
        [ ] Return value passing & caching
    [ ] Sketch out design for parallelized expression computation (e.g. in WebWorkers)
    [ ] Patch ExpressionTable into cartridge à la PropertyCoproduct



## Backlog

[ ] Margin & padding?
    [ ] Decide whether to support, e.g. is there a simpler alternative w/ existing pieces?
    [ ] Decide whether to support ONE or BOTH
[ ] Ellipse
[ ] Path
[ ] Frames: overflow scrolling
[ ] Should (can?) `align` be something like (Size::Percent, Size::Percent) instead of a less explicit (f64, f64)?
[ ] PoC on macOS, iOS, Android, Windows
[ ] Image primitive
    [ ] Hook into `piet`s image rendering
    [ ] Asset management
[ ] Gradients
    [ ] Multiple (stacked, polymorphic) fills
[ ] Production compilation
    [  ] Generation of RIL, feature-gating `designtime`
[ ] Packaging & imports
    [ ] Ensure that 3rd party components can be loaded via vanilla import mechanism
[ ] Mixed mode, Web
    [x] Rust -> JS data bridge
    [x] DOM pooling & recycling mechanism
    [ ] Text primitives + basic styling
    [ ] Native-layer clipping (accumulate clipping path for elements above DOM elements, communicate as Path to web layer for foreignObject + SVG clipping)
    [ ] Form controls
    [ ] ButtonNative (vs. ButtonGroup/ButtonContainer/ButtonFrame?) (or vs. a click event on any ol element)
    [ ] Text input
    [ ] Dropdown
[ ] JavaScript runtime
    [ ] First-class TypeScript support
    [ ] API design
        [ ] code-behind & decorator syntax
    [ ] Bindings to `runtime` API, plus IPC mechanism for triggering
[ ] Language server, syntax highlighting, IDE errors (VSCode, JetBrains)
[ ] Transform.shear
[ ] Audio/video components
    [ ] "headless" components
[ ] Expression pre-compiler
    [ ] Enforce uniqueness and valid node/var naming, e.g. for `my_node.var.name`
    [ ] Parser for custom expression lang
[ ] Debugging chassis
[ ] Perf-optimize Rectangle (assuming BezPath is inefficient)



```
Creative development environment
for makers of
graphical user interfaces
```




Lab journal:


