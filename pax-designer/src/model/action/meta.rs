use std::{borrow::BorrowMut, cell::RefCell, rc::Rc};

use pax_engine::{api::NodeContext, log};

use crate::model;

use super::Action;

thread_local! {
    static SHEDULED_ACTIONS: RefCell<Vec<Rc<dyn Action>>> = RefCell::default();
}

pub fn flush_sheduled_actions(ctx: &NodeContext) {
    model::with_action_context(ctx, |ctx| {
        let actions = SHEDULED_ACTIONS.with_borrow_mut(|sheduled| std::mem::take(sheduled));
        for action in actions {
            if let Err(e) = (*action).perform(ctx) {
                log::warn!("failed sheduled action: {e}");
            };
        }
    })
}

// Schedule an action to be performed next tick.
// Useful for when an action needs to be performed
// that modifies engine state for a node that doesn't exist
// yet or was just created.
pub struct Schedule {
    pub action: Rc<dyn Action>,
}

impl Action for Schedule {
    fn perform(&self, _ctx: &mut super::ActionContext) -> anyhow::Result<()> {
        SHEDULED_ACTIONS.with_borrow_mut(|sheduled| sheduled.push(Rc::clone(&self.action)));
        Ok(())
    }
}
