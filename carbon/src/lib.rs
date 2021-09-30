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
    [ ] Sizing + (browser) resizing support
        [ ] Transform.align
    [ ] Expression engine
        [x] variables, declaration & storage
        [x] node IDs
        [ ] summonables
            [ ] nested state access & access control (descendent vs ancestor)
            [ ] build-in vars like frame count
        [ ] parser & syntax (or MVP rust closures + manifest of deps)
    [ ] Text prefabs + support layer for Web adapter
        [ ] Clipping
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





