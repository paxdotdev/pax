use super::orm::MoveSelected;
use super::pointer::Pointer;
use super::{Action, ActionContext, CanUndo};
use crate::model::AppState;
use crate::model::{Tool, ToolVisual};
use crate::USERLAND_PROJECT_ID;
use anyhow::{anyhow, Result};
use pax_designtime::DesigntimeManager;
use pax_engine::rendering::{Point2D, TransformAndBounds};
use pax_std::types::Color;

pub struct ToolAction {
    pub tool: Tool,
    pub event: Pointer,
    pub point: Point2D,
}

impl Action for ToolAction {
    fn perform(self, ctx: &mut ActionContext) -> Result<CanUndo> {
        match self.tool {
            Tool::Rectangle => ctx.execute(RectangleTool {
                event: self.event,
                point: self.point,
            }),
            Tool::Pointer => ctx.execute(PointerTool {
                point: self.point,
                event: self.event,
            }),
        };

        Ok(CanUndo::No)
    }
}

pub struct RectangleTool {
    pub event: Pointer,
    pub point: Point2D,
}

impl Action for RectangleTool {
    fn perform(self, ctx: &mut ActionContext) -> Result<CanUndo> {
        match self.event {
            Pointer::Down => {
                ctx.app_state.tool_visual = Some(ToolVisual::Box {
                    p1: self.point,
                    p2: self.point,
                    stroke: Color::rgba(0.into(), 0.into(), 1.into(), 0.7.into()),
                    fill: Color::rgba(0.into(), 0.into(), 0.into(), 0.2.into()),
                });
            }
            Pointer::Move => {
                if let Some(ToolVisual::Box { ref mut p2, .. }) = ctx.app_state.tool_visual.as_mut()
                {
                    *p2 = self.point;
                }
            }
            Pointer::Up => {
                if let Some(ToolVisual::Box { p1, p2, .. }) = ctx.app_state.tool_visual.take() {
                    ctx.execute(super::orm::CreateRectangle {
                        origin: p1,
                        width: p2.x - p1.x,
                        height: p2.y - p1.y,
                    })?;
                }
            }
        }
        Ok(CanUndo::No)
    }
}

pub struct PointerTool {
    pub event: Pointer,
    pub point: Point2D,
}

impl Action for PointerTool {
    fn perform(self, ctx: &mut ActionContext) -> Result<CanUndo> {
        match self.event {
            Pointer::Down => {
                let all_elements_beneath_ray = ctx
                    .node_context
                    .runtime_context
                    .get_elements_beneath_ray((self.point.x, self.point.y), false, vec![]);
                if let Some(container) = ctx
                    .node_context
                    .runtime_context
                    .get_expanded_nodes_by_id(USERLAND_PROJECT_ID)
                    .first()
                {
                    if let Some(target) = all_elements_beneath_ray
                        .iter()
                        .find(|elem| elem.is_descendant_of(&container.id_chain))
                    {
                        // pax_engine::log::info!("Element hit! {:?}", target);
                        ctx.app_state.selected_template_node_id =
                            Some(target.instance_node.base().template_node_id);
                        let lp = target.layout_properties.borrow();
                        let corners = lp.as_ref().unwrap().computed_tab.corners();
                        let curr_pos = ctx.app_state.tool_visual = Some(ToolVisual::MovingNode {
                            grab_offset_x: self.point.x,
                            grab_offset_y: self.point.y,
                        });
                    } else {
                        ctx.app_state.tool_visual = Some(ToolVisual::Box {
                            p1: self.point,
                            p2: self.point,
                            stroke: Color::rgba(0.into(), 1.into(), 1.into(), 0.7.into()),
                            fill: Color::rgba(0.into(), 1.into(), 1.into(), 0.1.into()),
                        });
                    }
                } else {
                    panic!("somehow raycast didn't hit userland project");
                }
            }
            Pointer::Move => {
                if let Some(toolvisual) = ctx.app_state.tool_visual.clone() {
                    match toolvisual {
                        ToolVisual::Box { p2, .. } => {
                            let Some(ToolVisual::Box { ref mut p2, .. }) =
                                ctx.app_state.tool_visual.as_mut()
                            else {
                                unreachable!();
                            };
                            *p2 = self.point;
                        }
                        ToolVisual::MovingNode {
                            grab_offset_x,
                            grab_offset_y,
                        } => {
                            // TODO move relative to place
                            ctx.execute(MoveSelected { point: self.point });
                        }
                    }
                }
            }
            Pointer::Up => {
                if let Some(ToolVisual::Box { p1, p2, .. }) = ctx.app_state.tool_visual.take() {
                    // TODO get objects within rectangle from engine, and find their
                    // TemplateNode ids to set selection state.
                    let something_in_rectangle = true;
                    if something_in_rectangle {
                        ctx.app_state.selected_template_node_id = None;
                        //select things
                    }
                }
            }
        }
        Ok(CanUndo::No)
    }
}
