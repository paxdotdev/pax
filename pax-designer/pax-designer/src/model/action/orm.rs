use super::{Action, ActionContext, CanUndo};
use crate::model::AppState;
use anyhow::{anyhow, Result};
use pax_designtime::DesigntimeManager;
use pax_engine::{api::Size, rendering::Point2D, serde};

pub struct CreateRectangle {
    pub origin: Point2D,
    pub width: f64,
    pub height: f64,
}
impl Action for CreateRectangle {
    fn perform(self, ctx: &mut ActionContext) -> Result<CanUndo> {
        let mut dt = ctx.node_context.designtime.borrow_mut();
        let mut builder = dt.get_orm_mut().build_new_node(
            "pax_designer::pax_reexports::designer_project::Example".to_owned(),
            "pax_designer::pax_reexports::pax_std::primitives::Rectangle".to_owned(),
            "Rectangle".to_owned(),
            None,
        );

        //do stuff here later, and then save
        // TODO later make world space
        let gs_transform = ctx.app_state.screen_to_glass_transform;
        let p = gs_transform * self.origin;
        builder.set_property("x", &to_pixels(p.x))?;
        builder.set_property("y", &to_pixels(p.y))?;
        builder.set_property("width", &to_pixels(self.width))?;
        builder.set_property("height", &to_pixels(self.height))?;

        builder
            .save()
            .map_err(|e| anyhow!("could not save: {}", e))?;
        // pax_engine::log::debug!("saved new rect");

        Ok(CanUndo::Yes(Box::new(|ctx: &mut ActionContext| {
            // pax_engine::log::debug!("undid rect");
            let mut dt = ctx.node_context.designtime.borrow_mut();
            dt.get_orm_mut()
                .undo()
                .map_err(|e| anyhow!("cound't undo: {:?}", e))
        })))
    }
}

pub struct MoveSelected {
    pub point: Point2D,
}

impl Action for MoveSelected {
    fn perform(self, ctx: &mut ActionContext) -> Result<CanUndo> {
        let selected = ctx
            .app_state
            .selected_template_node_id
            .expect("executed action MoveSelected without a selected object");
        let mut dt = ctx.node_context.designtime.borrow_mut();

        let mut builder = dt.get_orm_mut().get_node(
            "pax_designer::pax_reexports::designer_project::Example",
            selected,
        );

        let gs_transform = ctx.app_state.screen_to_glass_transform;
        let p = gs_transform * self.point;
        builder.set_property("x", &to_pixels(p.x))?;
        builder.set_property("y", &to_pixels(p.y))?;
        builder
            .save()
            .map_err(|e| anyhow!("could not move thing: {}", e))?;

        Ok(CanUndo::Yes(Box::new(|ctx: &mut ActionContext| {
            // pax_engine::log::debug!("undid move");
            let mut dt = ctx.node_context.designtime.borrow_mut();
            dt.get_orm_mut()
                .undo()
                .map_err(|e| anyhow!("cound't undo: {:?}", e))
        })))
    }
}

fn to_pixels(v: f64) -> String {
    format!("{}px", v)
}
