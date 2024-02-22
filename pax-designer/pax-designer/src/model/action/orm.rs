use super::{Action, ActionContext, CanUndo};
use crate::math::AxisAlignedBox;
use crate::{
    math::BoxPoint,
    model::{
        math::coordinate_spaces::{Glass, World},
        AppState,
    },
};
use anyhow::{anyhow, Result};
use pax_designtime::DesigntimeManager;
use pax_engine::{
    api::Size,
    math::{Point2, Space, Vector2},
    serde,
};

pub struct CreateRectangle {
    pub origin: Point2<World>,
    pub dims: Vector2<World>,
}
impl Action for CreateRectangle {
    fn perform(self: Box<Self>, ctx: &mut ActionContext) -> Result<CanUndo> {
        let mut dt = ctx.engine_context.designtime.borrow_mut();
        let mut builder = dt.get_orm_mut().build_new_node(
            ctx.app_state.selected_component_id.clone(),
            "pax_designer::pax_reexports::pax_std::primitives::Rectangle".to_owned(),
            "Rectangle".to_owned(),
            None,
        );
        builder.set_property("x", &to_pixels(self.origin.x))?;
        builder.set_property("y", &to_pixels(self.origin.y))?;
        builder.set_property("width", &to_pixels(self.dims.x))?;
        builder.set_property("height", &to_pixels(self.dims.y))?;

        builder
            .save()
            .map_err(|e| anyhow!("could not save: {}", e))?;

        Ok(CanUndo::Yes(Box::new(|ctx: &mut ActionContext| {
            let mut dt = ctx.engine_context.designtime.borrow_mut();
            dt.get_orm_mut()
                .undo()
                .map_err(|e| anyhow!("cound't undo: {:?}", e))
        })))
    }
}

pub struct MoveSelected {
    pub point: Point2<World>,
}

impl Action for MoveSelected {
    fn perform(self: Box<Self>, ctx: &mut ActionContext) -> Result<CanUndo> {
        let selected = ctx
            .app_state
            .selected_template_node_id
            .expect("executed action MoveSelected without a selected object");
        let mut dt = ctx.engine_context.designtime.borrow_mut();

        let mut builder = dt
            .get_orm_mut()
            .get_node(&ctx.app_state.selected_component_id, selected);

        builder.set_property("x", &to_pixels(self.point.x))?;
        builder.set_property("y", &to_pixels(self.point.y))?;
        builder
            .save()
            .map_err(|e| anyhow!("could not move thing: {}", e))?;

        Ok(CanUndo::Yes(Box::new(|ctx: &mut ActionContext| {
            // pax_engine::log::debug!("undid move");
            let mut dt = ctx.engine_context.designtime.borrow_mut();
            dt.get_orm_mut()
                .undo()
                .map_err(|e| anyhow!("cound't undo: {:?}", e))
        })))
    }
}

pub struct ResizeSelected {
    pub attachment_point: Point2<BoxPoint>,
    pub position: Point2<World>,
}

impl Action for ResizeSelected {
    fn perform(self: Box<Self>, ctx: &mut ActionContext) -> Result<CanUndo> {
        let selected = ctx
            .app_state
            .selected_template_node_id
            .expect("executed action ResizeSelected without a selected object");
        let mut dt = ctx.engine_context.designtime.borrow_mut();
        let mut builder = dt
            .get_orm_mut()
            .get_node(&ctx.app_state.selected_component_id, selected);

        let bounds = ctx.selected_bounds().unwrap();

        let resize_anchor = ctx.world_transform() * bounds.lerp(self.attachment_point);

        let new_box = AxisAlignedBox::new(self.position, resize_anchor);
        pax_engine::log::info!("resizing box: {:?}", new_box);
        if self.attachment_point.y != 0.0 {
            builder.set_property("y", &to_pixels(new_box.top_left().y));
            builder.set_property("height", &to_pixels(new_box.height()));
        }

        if self.attachment_point.x != 0.0 {
            builder.set_property("x", &to_pixels(new_box.top_left().x));
            builder.set_property("width", &to_pixels(new_box.width()));
        }

        builder
            .save()
            .map_err(|e| anyhow!("could not move thing: {}", e))?;

        Ok(CanUndo::Yes(Box::new(|ctx: &mut ActionContext| {
            let mut dt = ctx.engine_context.designtime.borrow_mut();
            dt.get_orm_mut()
                .undo()
                .map_err(|e| anyhow!("cound't undo: {:?}", e))
        })))
    }
}

fn to_pixels(v: f64) -> String {
    format!("{}px", v)
}
