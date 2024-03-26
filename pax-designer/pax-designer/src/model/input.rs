use std::collections::HashMap;

use anyhow::{anyhow, Result};
use pax_designtime::DesigntimeManager;

use crate::{controls::toolbar, glass, llm_interface::OpenLLMPrompt};
use crate::model::action::orm::{UndoRequested, SerializeRequested};

use super::{
    action::{self, meta::ActionSet, orm::DeleteSelected, world, Action, ActionContext, CanUndo},
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
                (RawInput::K, InputEvent::OpenLLMPrompt),
                (RawInput::Delete, InputEvent::DeleteSelected),
                (RawInput::Backspace, InputEvent::DeleteSelected),
                (RawInput::Z, InputEvent::Undo),
                (RawInput::S, InputEvent::Serialize),
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
            (&InputEvent::OpenLLMPrompt, Dir::Down) => {
                Some(Box::new(OpenLLMPrompt { require_meta: true }))
            }
            (&InputEvent::DeleteSelected, Dir::Down) => Some(Box::new(DeleteSelected {})),
            (&InputEvent::Undo, Dir::Down) => Some(Box::new(UndoRequested {})),
            (&InputEvent::Serialize, Dir::Down) => Some(Box::new(SerializeRequested {})),
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
    Delete,
    Backspace,
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
    K,
    S,
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
            "k" => Self::K,
            "s" => Self::S,
            " " => Self::Space,
            "Control" => Self::Control,
            "=" => Self::Plus,
            "-" => Self::Minus,
            "Alt" => Self::Alt,
            "Meta" => Self::Meta,
            "Shift" => Self::Shift,
            "Delete" => Self::Delete,
            "Backspace" => Self::Backspace,
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
    OpenLLMPrompt,
    DeleteSelected,
    Undo,
    Serialize,
}
