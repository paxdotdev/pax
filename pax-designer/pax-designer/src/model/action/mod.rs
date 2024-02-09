use crate::model::AppState;
use anyhow::{anyhow, Result};
use pax_designtime::DesigntimeManager;

pub mod meta;
pub mod orm;
pub mod pointer;
pub mod tools;

type UndoFunc = dyn FnOnce(&mut ActionContext) -> Result<()>;

#[derive(Default)]
pub struct ActionManager {
    action_stack: Vec<Box<UndoFunc>>,
}

impl ActionManager {
    pub fn undo_last(
        &mut self,
        app_state: &mut AppState,
        designtime: &mut DesigntimeManager,
    ) -> Result<()> {
        let mut undo_fn = self.action_stack.pop().ok_or(anyhow!("undo stack embty"))?;
        let mut ctx = ActionContext {
            designtime,
            app_state,
            action_manager: self,
        };
        undo_fn(&mut ctx)
    }
}

pub trait Action {
    fn perform(self, ctx: &mut ActionContext) -> Result<Undoable>;
}

impl Action for Box<dyn Action> {
    fn perform(self, ctx: &mut ActionContext) -> Result<Undoable> {
        self.perform(ctx)
    }
}

pub enum Undoable {
    Yes(Box<UndoFunc>),
    No,
}

pub struct ActionContext<'a> {
    pub designtime: &'a mut DesigntimeManager,
    pub app_state: &'a mut AppState,
    pub action_manager: &'a mut ActionManager,
}

impl ActionContext<'_> {
    pub fn perform(&mut self, action: impl Action) -> Result<()> {
        if let Undoable::Yes(undo_fn) = action.perform(self)? {
            self.action_manager.action_stack.push(undo_fn);
        }
        Ok(())
    }
}
