use anyhow::Result;

use crate::DesigntimeManager;

#[derive(Default)]
pub struct ActionManager {
    action_stack: Vec<Box<dyn Action>>,
}

impl ActionManager {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn perform(
        &mut self,
        mut action: Box<dyn Action>,
        designtime: &mut DesigntimeManager,
    ) -> Result<()> {
        action.perform(designtime)?;
        self.action_stack.push(action);
        Ok(())
    }

    pub fn undo_last(
        &mut self,
        mut action: Box<dyn Action>,
        designtime: &mut DesigntimeManager,
    ) -> Result<()> {
        action.undo(designtime)?;
        self.action_stack.push(action);
        Ok(())
    }
}

pub trait Action {
    fn perform(&mut self, designtime: &mut DesigntimeManager) -> Result<()>;
    fn undo(&mut self, designtime: &mut DesigntimeManager) -> Result<()>;
}

struct CreateRectangle {}

impl Action for CreateRectangle {
    fn perform(&mut self, designtime: &mut DesigntimeManager) -> Result<()> {
        let builder = designtime.get_orm_mut().build_new_node(
            "component1".to_owned(),
            "..rectangle".to_owned(),
            "???".to_owned(),
            None,
        );
        //do stuff here
        let _ = builder;
        Ok(())
    }

    fn undo(&mut self, _designtime: &mut DesigntimeManager) -> Result<()> {
        todo!("undo rect creation")
    }
}

// "central hub" of actions.
// often triggered by FSM / input manager
// Can perform selection, can edit ORM
// Tracks undo stack for every action (including selection)
// Can reach into engine to perform ray-casting (and likely other) operations
//

// Decide: enum or trait for actions?
//  The action itself could either be inside a large match statement (enum)
//  or an impl fn on the trait

// Some known actions:
// ResolveTargets (under mouse coordinates, requires raycasting)
// PerformSelection
// ClearSelection

// Regarding undo, some actions may need to be batched — e.g. deselecting and then
// selecting elements.  The alternative would be to have atomic actions (DeselectAllAndSelect) that
// manage multiple logical things.
// Seems cleaner to start with the latter, and possibly to extend the undo mechanism to handle multiple actions
// at some future time.
