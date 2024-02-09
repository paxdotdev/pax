use std::collections::HashMap;

use anyhow::{anyhow, Result};
use pax_designtime::DesigntimeManager;

use super::{action::Action, Tool};

struct InputMapper {
    keymap_map: HashMap<RawInput, InputEvent>,
}

impl Default for InputMapper {
    fn default() -> Self {
        Self {
            keymap_map: HashMap::from([(
                RawInput::Key('r'),
                InputEvent::QuickUse(Tool::Rectangle),
            )]),
        }
    }
}

// This represents the actual input performed by the user
// to be rebindable in a settingsview
#[derive(PartialEq, Eq, Hash)]
pub enum RawInput {
    Key(char),
    Pointer(Dir),
}

// This represents the "actions" that can be taken by the user that could
// potentially be remapped to arbitrary keys. Only user input events: no
// internal message passing to be done using these tupes
pub enum InputEvent {
    SelectTool(Tool),
    QuickUse(Tool),
    MainModifierKey(Dir),
    Pointer(Dir),
}

#[derive(PartialEq, Eq, Hash)]
pub enum Dir {
    Down,
    Up,
}
