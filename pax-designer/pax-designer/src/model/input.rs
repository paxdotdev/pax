use std::collections::HashMap;

use anyhow::{anyhow, Result};
use pax_designtime::DesigntimeManager;

use crate::{controls::toolbar, glass};

use super::{
    action::{self, meta::ActionSet, world, Action, ActionContext, CanUndo},
    Component, Tool,
};

pub struct InputMapper {
    keymap: HashMap<RawInput, InputEvent>,
}

impl Default for InputMapper {
    fn default() -> Self {
        Self {
            //Default keymap, will be configurable in settings
            keymap: HashMap::from([
                (
                    RawInput::R,
                    InputEvent::SelectTool(Tool::CreateComponent(Component::Rectangle)),
                ),
                (RawInput::V, InputEvent::SelectTool(Tool::Pointer)),
                (RawInput::Control, InputEvent::Control),
                (RawInput::Space, InputEvent::Space),
                (RawInput::Plus, InputEvent::Plus),
                (RawInput::Minus, InputEvent::Minus),
                (RawInput::Alt, InputEvent::Alt),
                (RawInput::Meta, InputEvent::Meta),
                (RawInput::Shift, InputEvent::Shift),
            ]),
        }
    }
}

impl InputMapper {
    pub fn to_event(&self, input: RawInput) -> Option<&InputEvent> {
        self.keymap.get(&input)
    }

    pub fn to_action(&self, event: &InputEvent, dir: Dir) -> Option<Box<dyn Action>> {
        match (event, dir) {
            (&InputEvent::SelectTool(tool), Dir::Down) => {
                Some(Box::new(toolbar::SelectTool { tool }))
            }
            (&InputEvent::Plus, Dir::Down) => Some(Box::new(world::Zoom { closer: true })),
            (&InputEvent::Minus, Dir::Down) => Some(Box::new(world::Zoom { closer: false })),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub enum Dir {
    Down,
    Up,
}

// This represents the actual input performed by the user
// to be rebindable in a settingsview
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RawInput {
    R,
    V,
    Control,
    Space,
    Plus,
    Minus,
    Alt,
    Z,
    Meta,
    Shift,
}

// TODO make RawInput be what is returned by the engine itself, instead
// of performing conversion here
impl TryFrom<String> for RawInput {
    type Error = anyhow::Error;

    fn try_from(value: String) -> std::result::Result<Self, Self::Error> {
        Ok(match value.as_str() {
            "r" => Self::R,
            "v" => Self::V,
            "z" => Self::Z,
            " " => Self::Space,
            "Control" => Self::Control,
            "=" => Self::Plus,
            "-" => Self::Minus,
            "Alt" => Self::Alt,
            "Meta" => Self::Meta,
            "Shift" => Self::Shift,
            _ => return Err(anyhow!("no configured raw input mapping for {:?}", value)),
        })
    }
}

// This represents the "actions" that can be taken by the user that could
// potentially be remapped to arbitrary keys. Only user input events: no
// internal message passing to be done using these tupes
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum InputEvent {
    SelectTool(Tool),
    Control,
    Space,
    Plus,
    Minus,
    Alt,
    Meta,
    Shift,
}
