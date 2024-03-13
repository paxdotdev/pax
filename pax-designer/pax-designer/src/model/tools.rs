use std::ops::ControlFlow;
use std::rc::Rc;

use super::action::orm::{CreateComponent, MoveSelected};
use super::action::pointer::Pointer;
use super::action::{Action, ActionContext, CanUndo};
use super::input::InputEvent;
use crate::glass::RectTool;
use crate::math::AxisAlignedBox;
use crate::model::math::coordinate_spaces::Glass;
use crate::model::Tool;
use crate::model::{AppState, ToolBehaviour};
use crate::USERLAND_PROJECT_ID;
use anyhow::{anyhow, Result};
use pax_designtime::DesigntimeManager;
use pax_engine::api::Color;
use pax_engine::api::Size;
use pax_engine::math::Point2;
use pax_engine::math::Vector2;
use pax_engine::rendering::TransformAndBounds;
use pax_manifest::{PaxType, TemplateNodeId, TypeId, UniqueTemplateNodeIdentifier};

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
        let is_shift_key_down = ctx.app_state.keys_pressed.contains(&InputEvent::Shift);
        let is_alt_key_down = ctx.app_state.keys_pressed.contains(&InputEvent::Alt);
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
        offset: Vector2<Glass>,
    },
    Selecting {
        p1: Point2<Glass>,
        p2: Point2<Glass>,
    },
}

pub struct SelectNode {
    pub id: TemplateNodeId,
}

impl Action for SelectNode {
    fn perform(self: Box<Self>, ctx: &mut ActionContext) -> Result<CanUndo> {
        if !ctx.app_state.keys_pressed.contains(&InputEvent::Shift) {
            ctx.app_state.selected_template_node_ids.clear();
        }
        ctx.app_state.selected_template_node_ids.push(self.id);
        Ok(CanUndo::No)
    }
}

impl PointerTool {
    pub fn new(ctx: &mut ActionContext, point: Point2<Glass>) -> Self {
        if let Some(hit) = ctx.raycast_glass(point) {
            let node_id = hit.global_id().unwrap().get_template_node_id();
            let _ = ctx.execute(SelectNode { id: node_id });
            let origin_window = hit.origin().unwrap();
            let object_origin_glass = ctx.glass_transform() * origin_window;
            let offset = point - object_origin_glass;
            Self::Moving { offset }
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
            &mut PointerTool::Moving { offset } => {
                let world_point = ctx.world_transform() * (point - offset);
                ctx.execute(MoveSelected { point: world_point }).unwrap();
            }
            PointerTool::Selecting { ref mut p2, .. } => *p2 = point,
        }
        ControlFlow::Continue(())
    }

    fn pointer_up(&mut self, _point: Point2<Glass>, _ctx: &mut ActionContext) -> ControlFlow<()> {
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
