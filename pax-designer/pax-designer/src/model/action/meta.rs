use super::pointer::Pointer;
use super::{Action, ActionContext, CanUndo};
use crate::model::AppState;
use crate::model::Tool;
use anyhow::{anyhow, Context, Result};
use pax_designtime::DesigntimeManager;
use pax_engine::api::Color;

pub struct ActionSet {
    pub actions: Vec<Box<dyn Action>>,
}

impl Action for ActionSet {
    fn perform(self: Box<Self>, ctx: &mut ActionContext) -> Result<CanUndo> {
        let mut local_undo_stack = vec![];
        for action in self.actions {
            match action.perform(ctx) {
                Ok(undoable) => {
                    if let CanUndo::Yes(undo_fn) = undoable {
                        local_undo_stack.push(undo_fn);
                    }
                }
                Err(e) => {
                    for undo_fn in local_undo_stack {
                        undo_fn(ctx).with_context(|| {
                            "part of ActionSet rewind failed while rolling back"
                        })?;
                    }
                    return Err(anyhow!("ActionSet failed at: {:?}", e));
                }
            }
        }
        Ok(if local_undo_stack.is_empty() {
            CanUndo::No
        } else {
            CanUndo::Yes(Box::new(|ctx: &mut ActionContext| {
                for undo_fn in local_undo_stack {
                    undo_fn(ctx).with_context(|| "part of ActionSet rewind failed")?;
                }
                Ok(())
            }))
        })
    }
}
