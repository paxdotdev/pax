use super::{Action, ActionContext, CanUndo};
use crate::model::AppState;
use anyhow::{anyhow, Result};
use pax_designtime::DesigntimeManager;

pub struct CreateRectangle {}
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
        builder.set_property("x", "20%")?;
        builder.set_property("y", "20%")?;
        builder.set_property("widhh", "80%")?;
        builder.set_property("height", "80%")?;

        builder
            .save()
            .map_err(|e| anyhow!("could not save: {}", e))?;
        pax_engine::log::debug!("saved new rect");

        Ok(CanUndo::Yes(Box::new(|ctx: &mut ActionContext| {
            pax_engine::log::debug!("undid rect");
            let mut dt = ctx.node_context.designtime.borrow_mut();
            dt.get_orm_mut()
                .undo()
                .map_err(|e| anyhow!("cound't undo: {:?}", e))
        })))
    }
}
