use super::orm::MoveSelected;
use super::pointer::Pointer;
use super::{Action, ActionContext, CanUndo};
use crate::model::math::Glass;
use crate::model::AppState;
use crate::model::{Tool, ToolState};
use crate::USERLAND_PROJECT_ID;
use anyhow::{anyhow, Result};
use pax_designtime::DesigntimeManager;
use pax_engine::math::Point2;
use pax_engine::math::Vector2;
use pax_engine::rendering::TransformAndBounds;
use pax_std::types::Color;

pub struct ToolAction {
    pub event: Pointer,
    pub point: Point2<Glass>,
}

impl Action for ToolAction {
    fn perform(self, ctx: &mut ActionContext) -> Result<CanUndo> {
        match ctx.app_state.selected_tool {
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
    pub point: Point2<Glass>,
}

impl Action for RectangleTool {
    fn perform(self, ctx: &mut ActionContext) -> Result<CanUndo> {
        match self.event {
            Pointer::Down => {
                ctx.app_state.tool_state = ToolState::Box {
                    p1: self.point,
                    p2: self.point,
                    stroke: Color::rgba(0.into(), 0.into(), 1.into(), 0.7.into()),
                    fill: Color::rgba(0.into(), 0.into(), 0.into(), 0.2.into()),
                };
            }
            Pointer::Move => {
                if let ToolState::Box { ref mut p2, .. } = ctx.app_state.tool_state {
                    *p2 = self.point;
                }
            }
            Pointer::Up => {
                if let ToolState::Box { p1, p2, .. } = std::mem::take(&mut ctx.app_state.tool_state)
                {
                    ctx.execute(super::orm::CreateRectangle {
                        origin: p1,
                        width: p2.x - p1.x,
                        height: p2.y - p1.y,
                    })?;

                    ctx.app_state.selected_tool = Tool::Pointer;
                }
            }
        }
        Ok(CanUndo::No)
    }
}

pub struct PointerTool {
    pub event: Pointer,
    pub point: Point2<Glass>,
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
                        ctx.app_state.selected_template_node_id =
                            Some(target.instance_node.base().template_node_id);

                        let common_props = target.get_common_properties();
                        let common_props = common_props.borrow();
                        let lp = target.layout_properties.borrow();
                        let tab = &lp.as_ref().unwrap().computed_tab;
                        let p_anchor = Point2::new(
                            common_props
                                .anchor_x
                                .as_ref()
                                .map(|x| x.get().get_pixels(tab.bounds.0))
                                .unwrap_or(0.0),
                            common_props
                                .anchor_y
                                .as_ref()
                                .map(|y| y.get().get_pixels(tab.bounds.1))
                                .unwrap_or(0.0),
                        );
                        let p = tab.transform * p_anchor;
                        let delta = p - self.point;
                        let curr_pos = ctx.app_state.tool_state = ToolState::Movement {
                            x: delta.x,
                            y: delta.y,
                        };
                    } else {
                        ctx.app_state.tool_state = ToolState::Box {
                            p1: self.point,
                            p2: self.point,
                            stroke: Color::rgba(0.into(), 1.into(), 1.into(), 0.7.into()),
                            fill: Color::rgba(0.into(), 1.into(), 1.into(), 0.1.into()),
                        };
                    }
                } else {
                    panic!("somehow raycast didn't hit userland project");
                }
            }
            Pointer::Move => match ctx.app_state.tool_state {
                ToolState::Box { p2, .. } => {
                    let ToolState::Box { ref mut p2, .. } = ctx.app_state.tool_state else {
                        unreachable!();
                    };
                    *p2 = self.point;
                }
                ToolState::Movement { x, y } => {
                    ctx.execute(MoveSelected {
                        point: self.point + Vector2::new(x, y),
                    });
                }
                ToolState::Idle => (),
            },
            Pointer::Up => {
                if let ToolState::Box { p1, p2, .. } = std::mem::take(&mut ctx.app_state.tool_state)
                {
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
