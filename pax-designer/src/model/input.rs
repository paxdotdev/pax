use std::collections::{HashMap, HashSet};

use anyhow::{anyhow, Result};
use pax_designtime::DesigntimeManager;
use pax_engine::api::{borrow, borrow_mut, Fill, Interpolatable, Stroke};
use pax_engine::math::Vector2;
use pax_engine::pax_manifest::{UniqueTemplateNodeIdentifier, ValueDefinition};
use pax_engine::{log, CoercionRules, Property};

use crate::controls::toolbar::FinishCurrentTool;
use crate::model::action::orm::{RedoRequested, SerializeRequested, UndoRequested};
use crate::model::SelectionStateSnapshot;
use crate::{controls::toolbar, glass, llm_interface::SetLLMPromptState};

use super::action::orm::group_ungroup::{GroupNodes, GroupSelected, GroupType, UngroupSelected};
use super::action::orm::other::SwapFillStrokeAction;
use super::action::orm::space_movement::TranslateFromSnapshot;
use super::action::orm::tree_movement::{RelativeMove, RelativeMoveSelected};
use super::action::orm::{Copy, Paste};
use super::action::world::SelectAllInOpenContainer;
use super::read_app_state;
use super::{
    action::{self, orm::DeleteSelected, world, Action, ActionContext},
    Tool, ToolbarComponent,
};

impl Interpolatable for InputMapper {}

#[derive(Clone)]
pub struct InputMapper {
    modifier_map: HashMap<RawInput, ModifierKey>,
    key_map: Vec<((RawInput, HashSet<ModifierKey>), InputEvent)>,
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
                (RawInput::Z, ModifierKey::Z),
                (RawInput::Space, ModifierKey::Space),
            ]),
            //Default keymap, will be configurable in settings
            key_map: [
                // --- Select tools ---
                // rectangle
                (
                    (RawInput::R, HashSet::new()),
                    InputEvent::SelectTool(Tool::CreateComponent(ToolbarComponent::Rectangle)),
                ),
                (
                    (RawInput::M, HashSet::new()),
                    InputEvent::SelectTool(Tool::CreateComponent(ToolbarComponent::Rectangle)),
                ),
                // ellipse
                (
                    (RawInput::O, HashSet::new()),
                    InputEvent::SelectTool(Tool::CreateComponent(ToolbarComponent::Ellipse)),
                ),
                (
                    (RawInput::E, HashSet::new()),
                    InputEvent::SelectTool(Tool::CreateComponent(ToolbarComponent::Ellipse)),
                ),
                // pointers
                (
                    (RawInput::V, HashSet::new()),
                    InputEvent::SelectTool(Tool::PointerPercent),
                ),
                (
                    (RawInput::V, HashSet::from([ModifierKey::Shift])),
                    InputEvent::SelectTool(Tool::PointerPixels),
                ),
                // text
                (
                    (RawInput::T, HashSet::new()),
                    InputEvent::SelectTool(Tool::CreateComponent(ToolbarComponent::Text)),
                ),
                (
                    (RawInput::T, HashSet::from([ModifierKey::Shift])),
                    InputEvent::SelectTool(Tool::CreateComponent(ToolbarComponent::Textbox)),
                ),
                // button
                (
                    (RawInput::B, HashSet::new()),
                    InputEvent::SelectTool(Tool::CreateComponent(ToolbarComponent::Button)),
                ),
                // -- Group/ungroup ops
                (
                    (RawInput::L, HashSet::from([ModifierKey::Meta])),
                    InputEvent::Group(GroupType::Link),
                ),
                (
                    (RawInput::G, HashSet::from([ModifierKey::Meta])),
                    InputEvent::Group(GroupType::Group),
                ),
                (
                    (
                        RawInput::G,
                        HashSet::from([ModifierKey::Meta, ModifierKey::Shift]),
                    ),
                    InputEvent::Ungroup,
                ),
                // --- Copy/Paste ---
                (
                    (RawInput::C, HashSet::from([ModifierKey::Meta])),
                    InputEvent::Copy,
                ),
                (
                    (RawInput::V, HashSet::from([ModifierKey::Meta])),
                    InputEvent::Paste,
                ),
                // --- Undo and Redo ---
                (
                    (RawInput::Z, HashSet::from([ModifierKey::Meta])),
                    InputEvent::Undo,
                ),
                (
                    (
                        RawInput::Z,
                        HashSet::from([ModifierKey::Meta, ModifierKey::Shift]),
                    ),
                    InputEvent::Redo,
                ),
                // --- Serialize/save ---
                ((RawInput::S, HashSet::new()), InputEvent::Serialize),
                // --- Zoom ---
                (
                    (RawInput::Plus, HashSet::from([ModifierKey::Meta])),
                    InputEvent::ZoomIn,
                ),
                (
                    (RawInput::Minus, HashSet::from([ModifierKey::Meta])),
                    InputEvent::ZoomOut,
                ),
                // --- Movement between layers ---
                (
                    (RawInput::OpenSquareBracket, HashSet::from([])),
                    InputEvent::LayerMove(RelativeMove::BumpDown),
                ),
                (
                    (RawInput::CloseSquareBracket, HashSet::from([])),
                    InputEvent::LayerMove(RelativeMove::BumpUp),
                ),
                (
                    (
                        RawInput::OpenSquareBracket,
                        HashSet::from([ModifierKey::Meta]),
                    ),
                    InputEvent::LayerMove(RelativeMove::Bottom),
                ),
                (
                    (
                        RawInput::CloseSquareBracket,
                        HashSet::from([ModifierKey::Meta]),
                    ),
                    InputEvent::LayerMove(RelativeMove::Top),
                ),
                // --- LLM Prompt ---
                // (
                //     (RawInput::K, HashSet::from([ModifierKey::Meta])),
                //     InputEvent::OpenLLMPrompt,
                // ),
                // --- Deletion ---
                (
                    (RawInput::Delete, HashSet::new()),
                    InputEvent::DeleteSelected,
                ),
                (
                    (RawInput::Backspace, HashSet::new()),
                    InputEvent::DeleteSelected,
                ),
                // --- Nudge Objects ---
                (
                    (RawInput::ArrowRight, HashSet::new()),
                    InputEvent::Nudge(NudgeDir::Right),
                ),
                (
                    (RawInput::ArrowLeft, HashSet::new()),
                    InputEvent::Nudge(NudgeDir::Left),
                ),
                (
                    (RawInput::ArrowUp, HashSet::new()),
                    InputEvent::Nudge(NudgeDir::Up),
                ),
                (
                    (RawInput::ArrowDown, HashSet::new()),
                    InputEvent::Nudge(NudgeDir::Down),
                ),
                // --- Util ---
                (
                    (RawInput::X, HashSet::from([ModifierKey::Shift])),
                    InputEvent::SwapFillStroke,
                ),
                (
                    (RawInput::A, HashSet::from([ModifierKey::Meta])),
                    InputEvent::SelectAllInOpenContainer,
                ),
                (
                    (RawInput::Esc, HashSet::from([])),
                    InputEvent::FinishCurrentTool,
                ),
            ]
            .to_vec(),
        }
    }
}

impl InputMapper {
    pub fn to_event(
        &self,
        input: RawInput,
        dir: Dir,
        modifiers: Property<HashSet<ModifierKey>>,
    ) -> Option<&InputEvent> {
        if let Some(modifier) = self.modifier_map.get(&input) {
            modifiers.update(|modifiers| {
                // HACK: browser for some reason doesn't trigger key up for "z" when
                // meta (and possibly other control keys) are pressed. so always
                // toggle z to false whenever any of them are released.
                if modifier != &ModifierKey::Z && dir == Dir::Up {
                    modifiers.remove(&ModifierKey::Z);
                }
                match dir {
                    Dir::Down => modifiers.insert(*modifier),
                    Dir::Up => modifiers.remove(modifier),
                };
            });
        }
        let modifiers = modifiers.get();
        // find the key combination that matches all required keys,
        // and contains the largest number of required keys
        let res = self
            .key_map
            .iter()
            .filter(|((i, m), _)| i == &input && m.is_subset(&modifiers))
            .max_by_key(|((_, m), _)| m.len())
            .map(|(_, v)| v);
        res
    }

    pub fn to_action(&self, event: &InputEvent, dir: Dir) -> Option<Box<dyn Action>> {
        if dir == Dir::Up {
            return None;
        }
        match event {
            &InputEvent::Group(group_type) => Some(Box::new(GroupSelected { group_type })),
            &InputEvent::SelectTool(tool) => Some(Box::new(toolbar::SelectTool { tool })),
            InputEvent::ZoomIn => Some(Box::new(world::Zoom { closer: true })),
            InputEvent::ZoomOut => Some(Box::new(world::Zoom { closer: false })),
            InputEvent::OpenLLMPrompt => Some(Box::new(SetLLMPromptState(true))),
            InputEvent::DeleteSelected => Some(Box::new(DeleteSelected {})),
            InputEvent::Undo => Some(Box::new(UndoRequested)),
            InputEvent::Redo => Some(Box::new(RedoRequested)),
            InputEvent::Serialize => Some(Box::new(SerializeRequested {})),
            InputEvent::Copy => Some(Box::new({
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
            InputEvent::Paste => Some(Box::new({
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
            InputEvent::LayerMove(relative_move) => Some(Box::new(RelativeMoveSelected {
                relative_move: *relative_move,
            })),
            InputEvent::SwapFillStroke => Some(Box::new(SwapFillStrokeAction)),
            InputEvent::Ungroup => Some(Box::new(UngroupSelected {})),
            InputEvent::SelectAllInOpenContainer => Some(Box::new(SelectAllInOpenContainer)),
            InputEvent::FinishCurrentTool => Some(Box::new(FinishCurrentTool)),
            InputEvent::Nudge(n_dir) => {
                struct Nudge(NudgeDir);

                impl Action for Nudge {
                    fn perform(&self, ctx: &mut ActionContext) -> Result<()> {
                        let initial_selection: SelectionStateSnapshot =
                            (&ctx.derived_state.selection_state.get()).into();
                        const GLASS_PIXELS: f64 = 3.0;

                        let t = ctx.transaction("nudging selection");
                        t.run(|| {
                            TranslateFromSnapshot {
                                translation: match self.0 {
                                    NudgeDir::Up => Vector2::new(0.0, -GLASS_PIXELS),
                                    NudgeDir::Down => Vector2::new(0.0, GLASS_PIXELS),
                                    NudgeDir::Left => Vector2::new(-GLASS_PIXELS, 0.0),
                                    NudgeDir::Right => Vector2::new(GLASS_PIXELS, 0.0),
                                },
                                initial_selection: &initial_selection,
                            }
                            .perform(ctx)
                        })
                    }
                }
                Some(Box::new(Nudge(n_dir.clone())))
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
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
    A,
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
    X,
    OpenSquareBracket,
    CloseSquareBracket,
    Esc,
    ArrowRight,
    ArrowLeft,
    ArrowUp,
    ArrowDown,
}

// TODO make RawInput be what is returned by the engine itself, instead
// of performing conversion here
impl TryFrom<String> for RawInput {
    type Error = anyhow::Error;

    fn try_from(value: String) -> std::result::Result<Self, Self::Error> {
        Ok(match value.to_lowercase().as_str() {
            "a" => Self::A,
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
            "x" => Self::X,
            "[" => Self::OpenSquareBracket,
            "]" => Self::CloseSquareBracket,
            " " => Self::Space,
            "arrowright" => Self::ArrowRight,
            "arrowleft" => Self::ArrowLeft,
            "arrowup" => Self::ArrowUp,
            "arrowdown" => Self::ArrowDown,
            "control" => Self::Control,
            "=" => Self::Plus,
            "-" => Self::Minus,
            "alt" => Self::Alt,
            "meta" => Self::Meta,
            "shift" => Self::Shift,
            "delete" => Self::Delete,
            "backspace" => Self::Backspace,
            "escape" => Self::Esc,
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
    ZoomIn,
    ZoomOut,
    OpenLLMPrompt,
    DeleteSelected,
    Undo,
    Redo,
    Serialize,
    Copy,
    Paste,
    Group(GroupType),
    Ungroup,
    SwapFillStroke,
    LayerMove(RelativeMove),
    SelectAllInOpenContainer,
    FinishCurrentTool,
    Nudge(NudgeDir),
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum NudgeDir {
    Up,
    Down,
    Left,
    Right,
}

impl Interpolatable for ModifierKey {}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Copy)]
pub enum ModifierKey {
    Control,
    Alt,
    Shift,
    Meta,
    Space,
    // "zoom mode"
    Z,
}
