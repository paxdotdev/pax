#[macro_use]
extern crate lazy_static;

pub use kurbo::{Affine};
pub use piet::{Color, StrokeStyle, Error};

mod engine;
mod rendering;
mod expressions;
mod components;
mod primitives;
mod runtime;
mod timeline;
mod designer;

pub use crate::engine::*;
pub use crate::primitives::*;
pub use crate::rendering::*;
pub use crate::expressions::*;
pub use crate::components::*;
pub use crate::runtime::*;
pub use crate::timeline::*;

/*
Creative development environment
for makers of
graphical user interfaces

Creative dev env
for makers of GUIs
[ . . . . . ]
(design, build, and ship.)?

TODO:
=== HIGH
    [x] Refactor PoC code into multi-file, better structure
    [x] Refactor web chassis
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
    [ ] Documentation & usage
    [ ] Mixed mode, Web
        [x] Rust -> JS data bridge
        [x] DOM pooling & recycling mechanism
        [ ] Text primitives + basic styling
        [ ] Native-layer clipping (accumulate clipping path for elements above DOM elements, communicate as Path to web layer for foreignObject + SVG clipping)
        [ ] Form controls
            [ ] ButtonNative (vs. ButtonGroup/ButtonContainer/ButtonFrame?) (or vs. a click event on any ol element)
            [ ] Text input
            [ ] Dropdown
    [ ] Compiler & Render Tree Manager
        [ ] e2e macro hookup
            [ ] create sample project; manually expand macros
            [ ] write MVP macros
                [ ] First write a hello world with a hand-written macro-manifest-coproduct
                [ ] Coordinate on manifest & PropertiesCoproduct via filesystem->idempotent-side-effect trick
        [ ] Patching & refactor
            [ ] Wait until e2e sample project hookup — chance for evolving scope
            [ ] Patchable trait
            [ ] Design: RectangleProperties, RectanglePropertiesPatch, update API, etc.
        [ ] Template compilation
            [x] Syntax & file design
            [ ] Code-behind & default implementations
            [ ] Helpful compiler errors, line numbers
            [x] Sanity-check path forward to JS runtime
        [ ] Property compilation
            [x] Syntax & file design
            [x] Parser

        [ ] Expression compilation MVP
            [ ] Syntax & file design
        [ ] Method hookups
            [ ] Dispatcher
            [ ] Instantiation, reference management, enum ID + addressing for method definitions &
                invocations

    [ ] Hook up all relevant properties to Property

What's our expression language MVP?
 - `==`, `&&`, and `||`
 - Parenthetical grouping `(.*)`
 - Literals for strings, bools, ints, floats
 - Nested object references + injected context
 - 
    [ ] Refactors
        [x] Bundle Transform into "sugary transform," incl. origin & align; consider a separate transform_matrix property
        [x] Is there a way to better-DRY the shared logic across render-nodes?
            e.g. check out the `get_size` methods for Frame and Spread
        [x] Maybe related to above:  can we DRY the default properties for a render node?
            Perhaps a macro is the answer?
        [ ] Revisit ..Default::default() constructor pattern, field privacy, ergonomics
            - Maybe break out all design-time properties into Properties objects,
              where we do ..Default::default(), then do plain ol' constructors for the rest
        [ ] Update expression/injector ergonomics; perhaps take a pass at macro-ifying injection (and/or removing variadics)
        [ ] Should (can?) `align` be something like (Size::Percent, Size::Percent) instead of a less explicit (f64, f64)?
            Same with `scale`
        [x] Can we do something better than `(Box<......>, Box<.......>)` for `Size`?
        [x] Rename various properties, e.g. bounding_dimens => bounds
        [x] Take a pass on references/ownership in render_render_tree — perhaps &Affine should transfer ownership instead, for example
        [ ] introduce a way to #derive `compute_in_place`
        [x] Better ergonomics for `wrap_render_node_ptr_into_list`
        [x] Evaluate whether to refactor the `unsafe` + PolymorphicType/PolymorphicData approach in expressions + scope data storage


=== MED
    [ ] Add an id to placeholders so they can be addressed via selectors?
        (e.g. to enable a component deep in a tree to expose a global/externally accessible placeholder
    [ ] Dependency management for expressions
        [ ] Declare runtime-accessible metadata around Properties
            objects (probably a macro)
        [ ] Automate registration of Properties objects into PropertiesCoproduct
            (probably the same macro as above)
        [ ] Support `descendent-properties-as-dependancies`, including
            [ ] Expose children (by id?) via dependency-lookup mechanism
                [ ] consider how/whether to handle virtual_children like repeated nodes
            [ ] Piece together properties/deps as eval'd from children
            during render tree traversal
        [ ] Disallow circular dependencies a la Excel
    [ ] Margin & padding?
        [ ] Decide whether to support, e.g. is there a simpler alternative w/ existing pieces?
        [ ] Decide whether to support ONE or BOTH
    [ ] Ellipse
    [ ] Path
    [ ] Frames: overflow scrolling
    [ ] PoC on macOS, iOS, Android
        [ ] Extricate Engine's dependency on WebRenderContext
    [ ] Image primitive
        [ ] Hook into `piet`s image rendering
        [ ] Asset management
    [ ] Gradients
        [ ] Multiple (stacked, polymorphic) fills
    [ ] Expressions
        [ ] manifests for properties
        [ ] dependency graph, smart traversal, circ. ref detection
        [ ] nested property access & figure out access control (descendent vs ancestor vs global+acyclic+(+private?))
        [ ] parser & syntax
        [ ] control flow ($repeat, $if)
        [ ] dependency graph + caching
    [ ] Tests
    [ ] Actions
        [ ] expose API for manipulating Properties via Actions
        [ ] Handle de/serialization with
    [ ] Authoring tool
        [ ] De/serialization to BESTful (HTML-ish, template) format
        [ ] Drawing tools
        [ ] Layout-building tools
=== LOW
    [ ] Transform.shear
    [ ] Audio/video components
        [ ] "headless" components
    [ ] Expression pre-compiler
        [ ] Enforce uniqueness and valid node/var naming, e.g. for `my_node.var.name`
        [ ] Parser for custom expression lang
    [ ] Debugging chassis
    [ ] Perf-optimize Rectangle (assuming BezPath is inefficient)
 */




/*

Scribble: can (should?) we achieve something like the following?

Sparse constructor pattern (ft. macro)

#derive[(Default)]
struct MyStruct {
    pub a: &str,
    pub b: &str,
    pub c: &str,
}

impl MyStruct {

    fn new() -> Self {

    }

    MyStruct { sparse!(MyStruct { a: "abc", c: "cde"}) }) //note the lack of `b`
}

 */



