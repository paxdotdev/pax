pub use kurbo::{Affine};
pub use piet::{Color, StrokeStyle, Error};

mod engine;
mod rendering;
mod expressions;
mod prefabs;

pub use crate::engine::*;
pub use crate::rendering::*;
pub use crate::expressions::*;
pub use crate::prefabs::*;

/*
Creative development environment
for makers of
graphical user interfaces

Creative dev env
for makers of GUIs
[ . . . . . ]

TODO:
=== HIGH
    [x] Refactor PoC code into multi-file, better structure
    [x] Refactor web chassis
    [x] Logging
    [x] Stroke, color, fill
    [x] Clean up warnings
    [x] Sizing
        [x] Browser resize support
        [x] None-sizing
        [x] Transform.align
    [x] Expression engine
        [x] variables, declaration & storage
        [x] node IDs
        [x] summonables
            [x] built-in vars like frame count
        [x] MVP rust closures + manifest of deps
    [ ] Layouts (stacks)
        [x] Decide `primitive` vs. userland `prefabs`
            `prefabs`
        [ ] Template mechanism for prefabs
        [ ] Control-flow `yield` for inputs/children
            [ ] Ensure path forward to userland `placeholders`
        [ ] Control-flow `repeat` for cells & dividers inside template
        [ ] Clipping & Frames
    [ ] Timelines, transitions, t9ables
    [ ] Documentation & usage
    [ ] Mixed mode
        [ ] Native layout
        [ ] Text primitives
        [ ] Native-layer clipping (accumulate clipping path for elements above DOM elements, communicate as Path to web layer for foreignObject + SVG clipping)
        [ ] Form controls
            [ ] ButtonNative (vs. ButtonGroup/ButtonContainer/ButtonFrame?) (or vs. a click event on any ol element)
            [ ] Text input
            [ ] Dropdown
    [ ] Hook up all relevant properties to Property
    [ ] Refactors
        [ ] Rename various properties, e.g. bounding_dimens => bounds
        [ ] Bundle Transform into "sugary transform," incl. origin & align; consider a separate transform_matrix property
        [ ] Take a pass on references/ownership in render_scene_graph — perhaps &Affine should transfer ownership instead, for example
=== MED
    [ ] Ellipse
    [ ] Path
    [x] Transform.origin
    [ ] PoC on iOS, Android
        [ ] Extricate Engine's dependency on WebRenderContext
    [ ] Image primitive
    [ ] Gradients
        [ ] Multiple (stacked, polymorphic) fills
    [ ] De/serializing for BESTful format
    [ ] Expressions
        [ ] dependency graph, smart traversal, circ. ref detection
        [ ] nested property access & figure out access control (descendent vs ancestor vs global+acyclic+(+private?))
        [ ] parser & syntax
        [ ] control flow ($repeat, $if)
        [ ] dependency graph + caching
    [ ] Tests
    [ ] State + Actions
        [ ] track and update custom states/variables
        [ ] expose API for manipulating state via Actions
    [ ] Authoring tool
        [ ] Drawing tools
        [ ] Layout-building tools
=== LOW
    [ ] Transform.shear
    [ ] Audio/video prefabs
        [ ] "headless" prefabs
    [ ] Expression pre-compiler
        [ ] Enforce uniqueness and valid node/var naming, e.g. for `my_node.var.name`
        [ ] Parser for custom expression lang
    [ ] Debugging chassis
    [ ] Perf-optimize Rectangle (assuming BezPath is inefficient)
 */





