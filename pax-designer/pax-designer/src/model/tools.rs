use std::ops::ControlFlow;
use std::rc::Rc;

use super::action::orm::{CreateComponent, SetBoxSelected};
use super::action::pointer::Pointer;
use super::action::{Action, ActionContext, CanUndo};
use super::input::InputEvent;
use crate::glass::RectTool;
use crate::math::coordinate_spaces::{Glass, World};
use crate::math::AxisAlignedBox;
use crate::model::Tool;
use crate::model::{AppState, ToolBehaviour};
use crate::USERLAND_PROJECT_ID;
use anyhow::{anyhow, Result};
use pax_designtime::DesigntimeManager;
use pax_engine::api::Color;
use pax_engine::api::Size;
use pax_engine::math::Point2;
use pax_engine::math::Vector2;
use pax_engine::{log, NodeLocal, Properties};
use pax_manifest::{PaxType, TemplateNodeId, TypeId, UniqueTemplateNodeIdentifier};
use pax_runtime_api::math::Transform2;
use pax_runtime_api::{Axis, Window};

pub struct CreateComponentTool {
    type_id: TypeId,
    origin: Point2<Glass>,
    bounds: AxisAlignedBox,
}

impl CreateComponentTool {
    pub fn new(_ctx: &mut ActionContext, point: Point2<Glass>, type_id: &TypeId) -> Self {
        Self {
            type_id: type_id.clone(),
            origin: point,
            bounds: AxisAlignedBox::new(Point2::default(), Point2::default()),
        }
    }
}

impl ToolBehaviour for CreateComponentTool {
    fn pointer_down(&mut self, _point: Point2<Glass>, _ctx: &mut ActionContext) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }

    fn pointer_move(
        &mut self,
        point: Point2<Glass>,
        ctx: &mut ActionContext,
    ) -> std::ops::ControlFlow<()> {
        let is_shift_key_down = ctx
            .app_state
            .keys_pressed
            .get()
            .contains(&InputEvent::Shift);
        let is_alt_key_down = ctx.app_state.keys_pressed.get().contains(&InputEvent::Alt);
        self.bounds = AxisAlignedBox::new(self.origin, self.origin + Vector2::new(1.0, 1.0))
            .morph_constrained(point, self.origin, is_alt_key_down, is_shift_key_down);
        ControlFlow::Continue(())
    }

    fn pointer_up(&mut self, point: Point2<Glass>, ctx: &mut ActionContext) -> ControlFlow<()> {
        self.pointer_move(point, ctx);
        let world_box = self
            .bounds
            .try_into_space(ctx.world_transform())
            .expect("only translate/scale");
        ctx.execute(CreateComponent {
            bounds: world_box,
            type_id: self.type_id.clone(),
        })
        .unwrap();
        ControlFlow::Break(())
    }

    fn keyboard(
        &mut self,
        _event: crate::model::input::InputEvent,
        _dir: crate::model::input::Dir,
        _ctx: &mut ActionContext,
    ) -> std::ops::ControlFlow<()> {
        ControlFlow::Continue(())
    }

    fn visualize(&self, glass: &mut crate::glass::Glass) {
        glass.is_rect_tool_active.set(true);
        glass.rect_tool.set(RectTool {
            x: Size::Pixels(self.bounds.top_left().x.into()),
            y: Size::Pixels(self.bounds.top_left().y.into()),
            width: Size::Pixels(self.bounds.width().into()),
            height: Size::Pixels(self.bounds.height().into()),
            stroke: Color::rgba(0.into(), 0.into(), 255.into(), 200.into()),
            fill: Color::rgba(0.into(), 0.into(), 255.into(), 30.into()),
        });
    }
}

pub enum PointerTool {
    Moving {
        mouse_offset_from_top_left: Vector2<Glass>,
        props: Properties,
    },
    Selecting {
        p1: Point2<Glass>,
        p2: Point2<Glass>,
    },
}

pub struct SelectNode {
    pub id: TemplateNodeId,
    //if true, deselects all other objects first
    pub overwrite: bool,
}

impl Action for SelectNode {
    fn perform(self: Box<Self>, ctx: &mut ActionContext) -> Result<CanUndo> {
        if self.overwrite
            || !ctx
                .app_state
                .keys_pressed
                .get()
                .contains(&InputEvent::Shift)
        {
            ctx.app_state.selected_template_node_ids.update(|ids| {
                ids.clear();
            });
        }
        ctx.app_state.selected_template_node_ids.update(|ids| {
            ids.push(self.id);
        });
        Ok(CanUndo::No)
    }
}

impl PointerTool {
    pub fn new(ctx: &mut ActionContext, point: Point2<Glass>) -> Self {
        if let Some(hit) = ctx.raycast_glass(point) {
            let node_id = hit.global_id().unwrap().get_template_node_id();
            let _ = ctx.execute(SelectNode {
                id: node_id,
                overwrite: false,
            });

            let transform = ctx.glass_transform() * hit.transform().unwrap_or_default().inverse();
            let origin = transform * Point2::new(0.0, 0.0);
            let mouse_offset_from_top_left = point - origin; // ok

            let props = hit.common_properties();
            Self::Moving {
                mouse_offset_from_top_left,
                props,
            }
        } else {
            Self::Selecting {
                p1: point,
                p2: point,
            }
        }
    }
}

impl ToolBehaviour for PointerTool {
    fn pointer_down(&mut self, _point: Point2<Glass>, _ctx: &mut ActionContext) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }

    fn pointer_move(&mut self, point: Point2<Glass>, ctx: &mut ActionContext) -> ControlFlow<()> {
        match self {
            &mut PointerTool::Moving {
                mouse_offset_from_top_left,
                ref props,
            } => {
                let new_top_left = point - mouse_offset_from_top_left;
                let new_world_top_left = ctx.world_transform() * new_top_left;
                let stage = ctx.app_state.stage.get();
                let width = props
                    .width
                    .evaluate((stage.width as f64, stage.height as f64), Axis::X);
                let height = props
                    .height
                    .evaluate((stage.width as f64, stage.height as f64), Axis::Y);
                let node_box = AxisAlignedBox::new(
                    new_world_top_left,
                    new_world_top_left + Vector2::new(width, height),
                );

                if let Err(e) = ctx.execute(SetBoxSelected {
                    node_box,
                    props,
                    ignore_coord: (false, false),
                }) {
                    pax_engine::log::error!("Error moving selected: {:?}", e);
                }
            }
            PointerTool::Selecting { ref mut p2, .. } => *p2 = point,
        }
        ControlFlow::Continue(())
    }

    fn pointer_up(&mut self, point: Point2<Glass>, ctx: &mut ActionContext) -> ControlFlow<()> {
        // move last little distance to pointer up position
        self.pointer_move(point, ctx);

        if let PointerTool::Selecting { .. } = self {
            // TODO select multiple objects
        }
        ControlFlow::Break(())
    }

    fn keyboard(
        &mut self,
        _event: crate::model::input::InputEvent,
        _dir: crate::model::input::Dir,
        _ctx: &mut ActionContext,
    ) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }

    fn visualize(&self, glass: &mut crate::glass::Glass) {
        if let PointerTool::Selecting { p1, p2 } = self {
            glass.is_rect_tool_active.set(true);
            glass.rect_tool.set(RectTool {
                x: Size::Pixels(p1.x.into()),
                y: Size::Pixels(p1.y.into()),
                width: Size::Pixels((p2.x - p1.x).into()),
                height: Size::Pixels((p2.y - p1.y).into()),
                stroke: Color::rgba(0.into(), 255.into(), 255.into(), 200.into()),
                fill: Color::rgba(0.into(), 255.into(), 255.into(), 30.into()),
            });
        }
    }
}
