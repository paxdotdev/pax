pub use kurbo::{Affine};
pub use piet::{Color, StrokeStyle, Error};

mod engine;
mod scene_graph;
mod rendering;

pub use crate::engine::*;
pub use crate::scene_graph::*;
pub use crate::rendering::*;

/*
TODO:
=== HIGH
    [x] Refactor PoC code into multi-file, better structure
    [x] Refactor web chassis
    [x] Logging
    [ ] Stroke, color, fill
    [ ] Sizing + resizing support
        [ ] Transform.align
    [ ] Transform.origin
    [ ] Ellipse
    [ ] Clean up warnings
    [ ] Expression engine
        [ ] state & scopes
        [ ] parser & syntax
        [ ] dependency graph
    [ ] Text prefabs + support layer for Web adapter
        [ ] Clipping
    [ ] Layouts (stacks)
        [ ] Clipping
    [ ] Timelines, transitions, t9ables
=== MED
    [ ] PoC on iOS, Android
        [ ] Extricate Engine's dependency on WebRenderContext
    [ ] Gradients
    [ ] De/serializing for BESTful format
    [ ] Tests
    [ ] Authoring tool
        [ ] Drawing tools
        [ ] Layout-building tools
=== LOW
    [ ] Transform.shear
    [ ] Debugging chassis
    [ ] Perf-optimize Rectangle (assuming BezPath is inefficient)
 */





