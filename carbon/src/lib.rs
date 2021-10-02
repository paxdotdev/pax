pub use kurbo::{Affine};
pub use piet::{Color, StrokeStyle, Error};

mod engine;
mod rendering;
mod expressions;

pub use crate::engine::*;
pub use crate::rendering::*;
pub use crate::expressions::*;

/*
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
        [x] Transform.origin
    [ ] Expression engine
        [x] variables, declaration & storage
        [x] node IDs
        [x] summonables
            [ ] nested state access & figure out access control (descendent vs ancestor vs global+acyclic+(private?))
            [x] built-in vars like frame count
        [x] MVP rust closures + manifest of deps
    [ ] Text prefabs + support layer for Web adapter
        [ ] Clipping
    [ ] Hook up all relevant properties to Property
    [ ] Layouts (stacks)
        [ ] Clipping
    [ ] Timelines, transitions, t9ables
=== MED
    [ ] Ellipse
    [ ] Transform.origin
    [ ] PoC on iOS, Android
        [ ] Extricate Engine's dependency on WebRenderContext
    [ ] Gradients
        [ ] Multiple (stacked, polymorphic) fills
    [ ] De/serializing for BESTful format
    [ ] Expressions
        [ ] dependency graph, smart traversal, circ. ref detection
        [ ] parser & syntax
        [ ] control flow ($repeat, $if)
        [ ] dependency graph + caching
    [ ] Tests
    [ ] Authoring tool
        [ ] Drawing tools
        [ ] Layout-building tools
=== LOW
    [ ] Transform.shear
    [ ] Expression pre-compiler
        [ ] Enforce uniqueness and valid node/var naming, e.g. for `my_node.var.name`
        [ ] Parser for custom expression lang
    [ ] Debugging chassis
    [ ] Perf-optimize Rectangle (assuming BezPath is inefficient)
 */





