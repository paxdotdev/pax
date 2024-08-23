use std::collections::HashMap;

use anyhow::{anyhow, Result};
use pax_designtime::DesigntimeManager;
use pax_engine::api::Interpolatable;
use pax_engine::Property;

use crate::model::action::orm::{RedoRequested, SerializeRequested, UndoRequested};
use crate::{controls::toolbar, glass, llm_interface::OpenLLMPrompt};

use super::action::orm::{Copy, Paste};
use super::read_app_state;
use super::{
    action::{self, orm::DeleteSelected, world, Action, ActionContext},
    Component, Tool,
};

impl Interpolatable for InputMapper {}

#[derive(Clone)]
pub struct InputMapper {
    modifier_map: HashMap<RawInput, ModifierKey>,
    key_map: HashMap<(RawInput, ModifierKeySet), InputEvent>,
}

impl Default for InputMapper {
    fn default() -> Self {
        Self {
            // Modifer key map. Most likely platform specific at some point,
            // might be configuratble in settings?
            modifier_map: HashMap::from([
                (RawInput::Control, ModifierKey::Control),
                (RawInput::Meta, ModifierKey::Meta),
                (RawInput::Shift, ModifierKey::Shift),
                (RawInput::Alt, ModifierKey::Alt),
                (RawInput::Space, ModifierKey::Space),
            ]),
            //Default keymap, will be configurable in settings
            key_map: HashMap::from([
                // --- Select tools ---
                // rectangle
                (
                    (RawInput::R, ModifierKeySet::none()),
                    InputEvent::SelectTool(Tool::CreateComponent(Component::Rectangle)),
                ),
                (
                    (RawInput::M, ModifierKeySet::none()),
                    InputEvent::SelectTool(Tool::CreateComponent(Component::Rectangle)),
                ),
                // ellipse
                (
                    (RawInput::O, ModifierKeySet::none()),
                    InputEvent::SelectTool(Tool::CreateComponent(Component::Ellipse)),
                ),
                (
                    (RawInput::E, ModifierKeySet::none()),
                    InputEvent::SelectTool(Tool::CreateComponent(Component::Ellipse)),
                ),
                // pointers
                (
                    (RawInput::V, ModifierKeySet::none()),
                    InputEvent::SelectTool(Tool::PointerPercent),
                ),
                (
                    (RawInput::V, ModifierKey::Shift.into()),
                    InputEvent::SelectTool(Tool::PointerPixels),
                ),
                // text
                (
                    (RawInput::T, ModifierKeySet::none()),
                    InputEvent::SelectTool(Tool::CreateComponent(Component::Text)),
                ),
                (
                    (RawInput::T, ModifierKey::Shift.into()),
                    InputEvent::SelectTool(Tool::CreateComponent(Component::Textbox)),
                ),
                // button
                (
                    (RawInput::B, ModifierKeySet::none()),
                    InputEvent::SelectTool(Tool::CreateComponent(Component::Button)),
                ),
                // -- Group/ungroup ops
                (
                    (RawInput::L, ModifierKey::Meta.into()),
                    InputEvent::ToggleGroup(GroupType::Link),
                ),
                (
                    (RawInput::G, ModifierKey::Meta.into()),
                    InputEvent::ToggleGroup(GroupType::Group),
                ),
                // --- Copy/Paste ---
                ((RawInput::C, ModifierKey::Meta.into()), InputEvent::Copy),
                ((RawInput::V, ModifierKey::Meta.into()), InputEvent::Paste),
                // --- Undo and Redo ---
                ((RawInput::Z, ModifierKey::Meta.into()), InputEvent::Undo),
                (
                    (
                        RawInput::Z,
                        ModifierKeySet {
                            shift: true,
                            meta: true,
                            ..Default::default()
                        },
                    ),
                    InputEvent::Redo,
                ),
                // --- Serialize/save ---
                ((RawInput::S, ModifierKeySet::none()), InputEvent::Serialize),
                // --- Zoom ---
                (
                    (RawInput::Plus, ModifierKey::Meta.into()),
                    InputEvent::ZoomIn,
                ),
                (
                    (RawInput::Minus, ModifierKey::Meta.into()),
                    InputEvent::ZoomOut,
                ),
                // --- LLM Prompt ---
                (
                    (RawInput::K, ModifierKey::Meta.into()),
                    InputEvent::OpenLLMPrompt,
                ),
                // --- Deletion ---
                (
                    (RawInput::Delete, ModifierKeySet::none()),
                    InputEvent::DeleteSelected,
                ),
                (
                    (RawInput::Backspace, ModifierKeySet::none()),
                    InputEvent::DeleteSelected,
                ),
            ]),
        }
    }
}

impl InputMapper {
    pub fn to_event(
        &self,
        input: RawInput,
        dir: Dir,
        modifiers: Property<ModifierKeySet>,
    ) -> Option<&InputEvent> {
        if let Some(modifier) = self.modifier_map.get(&input) {
            let state = match dir {
                Dir::Down => true,
                Dir::Up => false,
            };
            modifiers.update(|modifiers| match modifier {
                ModifierKey::Control => modifiers.control = state,
                ModifierKey::Alt => modifiers.alt = state,
                ModifierKey::Shift => modifiers.shift = state,
                ModifierKey::Meta => modifiers.meta = state,
                ModifierKey::Space => modifiers.space = state,
            });
            None
        } else {
            let modifiers = modifiers.get();
            self.key_map.get(&(input, modifiers))
        }
    }

    pub fn to_action(&self, event: &InputEvent, dir: Dir) -> Option<Box<dyn Action>> {
        match (event, dir) {
            (&InputEvent::SelectTool(tool), Dir::Down) => {
                Some(Box::new(toolbar::SelectTool { tool }))
            }
            (&InputEvent::ZoomIn, Dir::Down) => Some(Box::new(world::Zoom { closer: true })),
            (&InputEvent::ZoomOut, Dir::Down) => Some(Box::new(world::Zoom { closer: false })),
            (&InputEvent::OpenLLMPrompt, Dir::Down) => Some(Box::new(OpenLLMPrompt)),
            (&InputEvent::DeleteSelected, Dir::Down) => Some(Box::new(DeleteSelected {})),
            (&InputEvent::Undo, Dir::Down) => Some(Box::new(UndoRequested)),
            (&InputEvent::Redo, Dir::Down) => Some(Box::new(RedoRequested)),
            (&InputEvent::Serialize, Dir::Down) => Some(Box::new(SerializeRequested {})),
            (&InputEvent::Copy, Dir::Down) => Some(Box::new({
                struct CopySelected;
                impl Action for CopySelected {
                    fn perform(&self, ctx: &mut ActionContext) -> Result<()> {
                        let ids = ctx.app_state.selected_template_node_ids.get();
                        let subtrees = Copy { ids: &ids }.perform(ctx)?;
                        ctx.app_state.clip_board.set(subtrees);
                        Ok(())
                    }
                }
                CopySelected
            })),
            (&InputEvent::Paste, Dir::Down) => Some(Box::new({
                struct PasteClipboard;

                impl Action for PasteClipboard {
                    fn perform(&self, ctx: &mut ActionContext) -> Result<()> {
                        let t = ctx.transaction("paste");
                        let subtrees = ctx.app_state.clip_board.get();
                        t.run(|| {
                            Paste {
                                subtrees: &subtrees,
                            }
                            .perform(ctx)
                        })
                        .map(|_| ())
                    }
                }
                PasteClipboard
            })),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
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
    C,
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
    M,
    O,
    E,
    T,
    B,
    L,
    G,
}

// TODO make RawInput be what is returned by the engine itself, instead
// of performing conversion here
impl TryFrom<String> for RawInput {
    type Error = anyhow::Error;

    fn try_from(value: String) -> std::result::Result<Self, Self::Error> {
        Ok(match value.to_lowercase().as_str() {
            "r" => Self::R,
            "m" => Self::M,
            "v" => Self::V,
            "z" => Self::Z,
            "k" => Self::K,
            "s" => Self::S,
            "c" => Self::C,
            "o" => Self::O,
            "e" => Self::E,
            "t" => Self::T,
            "b" => Self::B,
            "l" => Self::L,
            "g" => Self::G,
            " " => Self::Space,
            "control" => Self::Control,
            "=" => Self::Plus,
            "-" => Self::Minus,
            "alt" => Self::Alt,
            "meta" => Self::Meta,
            "shift" => Self::Shift,
            "delete" => Self::Delete,
            "backspace" => Self::Backspace,
            _ => return Err(anyhow!("no configured raw input mapping for {:?}", value)),
        })
    }
}

impl Interpolatable for InputEvent {}
// This represents the "actions" that can be taken by the user that could
// potentially be remapped to arbitrary keys. Only user input events: no
// internal message passing to be done using these tupes
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum InputEvent {
    SelectTool(Tool),
    Space,
    ZoomIn,
    ZoomOut,
    OpenLLMPrompt,
    DeleteSelected,
    Undo,
    Redo,
    Serialize,
    Copy,
    Paste,
    ToggleLinkGroup,
    ToggleGroup(GroupType),
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum GroupType {
    Link,
    Group,
}

impl Interpolatable for ModifierKeySet {}

#[derive(Clone, PartialEq, Eq)]
pub enum ModifierKey {
    Control,
    Alt,
    Shift,
    Meta,
    Space,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
pub struct ModifierKeySet {
    pub control: bool,
    pub alt: bool,
    pub meta: bool,
    pub shift: bool,
    pub space: bool,
}

impl From<ModifierKey> for ModifierKeySet {
    fn from(value: ModifierKey) -> Self {
        Self {
            control: value == ModifierKey::Control,
            alt: value == ModifierKey::Alt,
            meta: value == ModifierKey::Meta,
            shift: value == ModifierKey::Shift,
            space: value == ModifierKey::Space,
        }
    }
}

impl ModifierKeySet {
    pub fn none() -> Self {
        Self {
            control: false,
            alt: false,
            meta: false,
            shift: false,
            space: false,
        }
    }
}
