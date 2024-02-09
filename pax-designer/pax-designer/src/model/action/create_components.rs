use anyhow::{anyhow, Result};
use pax_designtime::DesigntimeManager;
use super::{UndoableAction, ActionContext, Action};
use crate::model::AppState;


pub struct CreateRectangle {}
impl UndoableAction for CreateRectangle {
    fn perform(&mut self, ctx: &mut ActionContext) -> Result<()> {
        let mut builder = ctx.designtime.get_orm_mut().build_new_node(
            "pax_designer::pax_reexports::designer_project::Example".to_owned(),
            "pax_designer::pax_reexports::pax_std::Rectangle".to_owned(),
            "Rectangle".to_owned(),
            None,
        );
        //do stuff here later, and then save
        builder.set_property("x", "20%")?;
        builder.set_property("y", "20%")?;
        builder.set_property("width", "80%")?;
        builder.set_property("height", "80%")?;

        builder.save().map_err(|e| anyhow!("could not save: {}", e))?;
        pax_lang::api::log("saved new rect");
        Ok(())
    }

    fn undo(&mut self, ctx: &mut ActionContext) -> Result<()> {
        pax_lang::api::log("undid rect creation");
        Ok(())
    }
}

impl From<CreateRectangle> for Action {
    fn from(value: CreateRectangle) -> Self {
        Action::Undoable(Box::new(value))
    }
}

