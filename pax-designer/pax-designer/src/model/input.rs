use std::collections::HashMap;

use anyhow::{anyhow, Result};
use pax_designtime::DesigntimeManager;

use crate::controls::toolbar;

use super::{
    action::{self, meta::ActionSet, Action, ActionContext, CanUndo},
    Tool,
};

struct InputMapper {
    keymap: HashMap<RawInput, InputEvent>,
}

impl Default for InputMapper {
    fn default() -> Self {
        Self {
            keymap: HashMap::from([
                (RawInput::Key('r'), InputEvent::SelectTool(Tool::Rectangle)),
                (RawInput::Key('v'), InputEvent::SelectTool(Tool::Pointer)),
            ]),
        }
    }
}

struct MainModiferKeyAction {
    dir: Dir,
}

impl Action for MainModiferKeyAction {
    fn perform(self, ctx: &mut ActionContext) -> Result<CanUndo> {
        ctx.app_state.main_mod_key = match self.dir {
            Dir::Down => true,
            Dir::Up => false,
        };
        Ok(CanUndo::No)
    }
}

impl InputMapper {
    pub fn to_action(&self, input: RawInput, dir: Dir) -> Option<Box<dyn Action>> {
        let event = self.keymap.get(&input)?;
        Some(match (event, dir) {
            (&InputEvent::SelectTool(tool), Dir::Down) => Box::new(toolbar::SelectTool { tool }),
            (InputEvent::MainModifierKey, dir) => Box::new(MainModiferKeyAction { dir }),
            (InputEvent::Pointer, Dir::Down) => todo!(),
            (InputEvent::Pointer, Dir::Up) => todo!(),
            _ => return None,
        })
    }
}

pub enum Dir {
    Down,
    Up,
}

// This represents the actual input performed by the user
// to be rebindable in a settingsview
#[derive(PartialEq, Eq, Hash)]
pub enum RawInput {
    Key(char),
    LeftMouse,
    RightMouse,
    MiddleMouse,
}

// This represents the "actions" that can be taken by the user that could
// potentially be remapped to arbitrary keys. Only user input events: no
// internal message passing to be done using these tupes
pub enum InputEvent {
    SelectTool(Tool),
    MainModifierKey,
    Pointer,
}
