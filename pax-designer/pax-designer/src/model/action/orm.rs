use super::{Action, ActionContext, CanUndo};
use crate::model::{
    math::coordinate_spaces::{Glass, World},
    AppState,
};
use anyhow::{anyhow, Result};
use pax_designtime::DesigntimeManager;
use pax_engine::{
    api::Size,
    math::{Point2, Vector2},
    serde,
};

pub struct CreateRectangle {
    pub origin: Point2<World>,
    pub dims: Vector2<World>,
}
impl Action for CreateRectangle {
    fn perform(self, ctx: &mut ActionContext) -> Result<CanUndo> {
        let mut dt = ctx.engine_context.designtime.borrow_mut();
        let mut builder = dt.get_orm_mut().build_new_node(
            "pax_designer::pax_reexports::designer_project::Example".to_owned(),
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
        // pax_engine::log::debug!("saved new rect");

        Ok(CanUndo::Yes(Box::new(|ctx: &mut ActionContext| {
            // pax_engine::log::debug!("undid rect");
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
    fn perform(self, ctx: &mut ActionContext) -> Result<CanUndo> {
        let selected = ctx
            .app_state
            .selected_template_node_id
            .expect("executed action MoveSelected without a selected object");
        let mut dt = ctx.engine_context.designtime.borrow_mut();

        let mut builder = dt.get_orm_mut().get_node(
            "pax_designer::pax_reexports::designer_project::Example",
            selected,
        );

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

fn to_pixels(v: f64) -> String {
    format!("{}px", v)
}
