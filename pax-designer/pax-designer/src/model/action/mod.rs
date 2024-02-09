use crate::model::AppState;
use anyhow::{anyhow, Result};
use pax_designtime::DesigntimeManager;

pub mod create_components;
pub mod pointer_events;
pub mod tool_events;

#[derive(Default)]
pub struct ActionManager {
    action_stack: Vec<Box<dyn UndoableAction>>,
}

impl ActionManager {
    pub fn undo_last(
        &mut self,
        app_state: &mut AppState,
        designtime: &mut DesigntimeManager,
    ) -> Result<()> {
        let mut action = self.action_stack.pop().ok_or(anyhow!("undo stack embty"))?;
        let mut ctx = ActionContext {
            designtime,
            app_state,
            action_manager: self,
        };
        action.undo(&mut ctx)?;
        Ok(())
    }
}

pub enum Action {
    Undoable(Box<dyn UndoableAction>),
    Oneshot(Box<dyn OneshotAction>),
}

pub trait UndoableAction {
    fn perform(&mut self, ctx: &mut ActionContext) -> Result<()>;
    fn undo(&mut self, ctx: &mut ActionContext) -> Result<()>;
}

pub trait OneshotAction {
    fn perform(&mut self, ctx: &mut ActionContext) -> Result<()>;
}

pub struct ActionContext<'a> {
    pub designtime: &'a mut DesigntimeManager,
    pub app_state: &'a mut AppState,
    pub action_manager: &'a mut ActionManager,
}

impl ActionContext<'_> {
    pub fn perform(&mut self, action: impl Into<Action>) -> Result<()> {
        match action.into() {
            Action::Undoable(mut a) => {
                let res = a.perform(self)?;
                self.action_manager.action_stack.push(a);
                Ok(res)
            }
            Action::Oneshot(mut a) => a.perform(self),
        }
    }
}
