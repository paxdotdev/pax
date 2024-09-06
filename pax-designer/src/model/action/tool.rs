use std::{cell::RefCell, rc::Rc};

use pax_engine::log;

use crate::model::ToolBehavior;

use super::Action;

pub struct SetToolBehaviour(pub Option<Rc<RefCell<dyn ToolBehavior>>>);

impl Action for SetToolBehaviour {
    fn perform(&self, ctx: &mut super::ActionContext) -> anyhow::Result<()> {
        let current = ctx.app_state.tool_behavior.get();
        if let Some(current) = current {
            if let Err(e) = current.borrow_mut().finish(ctx) {
                log::warn!("tool finish failed: {e}");
            }
        }
        ctx.app_state
            .tool_behavior
            .set(self.0.as_ref().map(Rc::clone));
        Ok(())
    }
}
