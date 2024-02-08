use anyhow::{anyhow, Result};

use crate::PaxManifestORM;

#[derive(Default)]
pub struct ActionManager {
    action_stack: Vec<Box<dyn Action>>,
}

impl ActionManager {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn perform(&mut self, mut action: Box<dyn Action>, orm: &mut PaxManifestORM) -> Result<()> {
        action.perform(orm)?;
        self.action_stack.push(action);
        Ok(())
    }

    pub fn undo_last(
        &mut self,
        mut action: Box<dyn Action>,
        orm: &mut PaxManifestORM,
    ) -> Result<()> {
        action.undo(orm)?;
        self.action_stack.push(action);
        Ok(())
    }
}

pub trait Action {
    fn perform(&mut self, orm: &mut PaxManifestORM) -> Result<()>;
    fn undo(&mut self, orm: &mut PaxManifestORM) -> Result<()>;
}

pub struct CreateRectangle {}

impl Action for CreateRectangle {
    fn perform(&mut self, orm: &mut PaxManifestORM) -> Result<()> {
        let mut builder = orm.build_new_node(
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

        builder.save().map_err(|e| anyhow!("could save: {}", e))?;
        Err(anyhow!("successfully created rect!"))
    }

    fn undo(&mut self, _orm: &mut PaxManifestORM) -> Result<()> {
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
