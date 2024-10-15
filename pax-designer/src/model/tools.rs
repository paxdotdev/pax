mod create_component_tool;
mod moving_tool;
mod multi_select_tool;
mod paintbrush_tool;
pub mod tool_plugins;
mod zoom_to_fit_tool;

use std::ops::ControlFlow;

pub use create_component_tool::*;
pub use moving_tool::*;
pub use multi_select_tool::*;
pub use paintbrush_tool::*;
pub use zoom_to_fit_tool::*;

use anyhow::{anyhow, Result};

use super::{
    action::ActionContext,
    input::{Dir, InputEvent},
};
use crate::{glass::ToolVisualizationState, math::coordinate_spaces::Glass};
use pax_engine::{math::Point2, Property};

pub trait ToolBehavior {
    fn pointer_down(&mut self, point: Point2<Glass>, ctx: &mut ActionContext) -> ControlFlow<()>;
    fn pointer_move(&mut self, point: Point2<Glass>, ctx: &mut ActionContext) -> ControlFlow<()>;
    fn pointer_up(&mut self, point: Point2<Glass>, ctx: &mut ActionContext) -> ControlFlow<()>;
    /// called before this tools get's replaced by another one: for example to commit text
    /// when the TextEdit tool get's replaced.
    fn finish(&mut self, ctx: &mut ActionContext) -> Result<()>;
    fn keyboard(&mut self, event: InputEvent, dir: Dir, ctx: &mut ActionContext)
        -> ControlFlow<()>;
    fn get_visual(&self) -> Property<ToolVisualizationState>;
}
