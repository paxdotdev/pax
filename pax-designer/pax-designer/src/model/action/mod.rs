use crate::model::AppState;
use anyhow::{anyhow, Result};
use pax_designtime::DesigntimeManager;

pub mod meta;
pub mod orm;
pub mod pointer;
pub mod tools;

type UndoFunc = dyn FnOnce(&mut ActionContext) -> Result<()>;

#[derive(Default)]
pub struct UndoStack {
    stack: Vec<Box<UndoFunc>>,
}

pub trait Action {
    fn perform(self, ctx: &mut ActionContext) -> Result<CanUndo>;
}

impl Action for Box<dyn Action> {
    fn perform(self, ctx: &mut ActionContext) -> Result<CanUndo> {
        self.perform(ctx)
    }
}

pub enum CanUndo {
    Yes(Box<UndoFunc>),
    No,
}

pub struct ActionContext<'a> {
    pub designtime: &'a mut DesigntimeManager,
    pub app_state: &'a mut AppState,
    pub undo_stack: &'a mut UndoStack,
}

impl ActionContext<'_> {
    pub fn perform(&mut self, action: impl Action) -> Result<()> {
        if let CanUndo::Yes(undo_fn) = action.perform(self)? {
            self.undo_stack.stack.push(undo_fn);
        }
        Ok(())
    }

    pub fn undo_last(&mut self) -> Result<()> {
        let mut undo_fn = self
            .undo_stack
            .stack
            .pop()
            .ok_or(anyhow!("undo stack embty"))?;
        undo_fn(self)
    }
}
