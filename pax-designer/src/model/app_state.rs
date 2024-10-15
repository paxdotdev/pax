use super::input::InputMapper;
use super::input::ModifierKey;
use super::SelectionState;
use super::ToolBehavior;
use crate::math::coordinate_spaces::Glass;
use crate::math::coordinate_spaces::World;
use crate::math::SizeUnit;
use pax_designtime::orm::SubTrees;
use pax_engine::api::Color;
use pax_engine::api::Interpolatable;
use pax_engine::api::Window;
use pax_engine::math::Point2;
use pax_engine::math::Transform2;
use pax_engine::pax;
use pax_engine::pax_manifest::TemplateNodeId;
use pax_engine::pax_manifest::TypeId;
use pax_engine::pax_manifest::UniqueTemplateNodeIdentifier;
use pax_engine::NodeInterface;
use pax_engine::Property;
use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

/// Represents the global source-of-truth for the designer.
/// Invalid if any of the bellow :INVALID_IF: statements hold true.
/// NOTE: Only add a new field to this struct if:
/// 1. There is no way of computing the sought after value from a combination
/// of AppState and Manifest information (if this is true, make it part of the
/// DerivedAppState struct)
/// 2. An effort has been made to reduce the number of invalid states:
/// use an enum instead of an usize if there exists a fixed set of of options,
/// and try to drill down and add state in inner AppState variables if it makes
/// sense (for example try to slot things like state of the currently used tool
/// into the tool_state variable, since no two toolstates can exist at the same time)
/// 3. The field prepresents a state that ideally would be saved between design
/// sessions. for the same user on save/load. (There are some exceptions atm,
/// like mouse position - but these should be moved)
#[derive(Default)]
#[deny(missing_docs)]
pub struct AppState {
    //---------------global-----------------
    /// The project mode (playing/editing)
    /// INVALID_IF: no invalid states
    pub project_mode: Property<ProjectMode>,
    /// Unit mode (are we drawing in px or %)
    /// INVALID_IF: no invalid states
    pub unit_mode: Property<SizeUnit>,
    /// The component currently being viewed and edited in the glass
    /// INVALID_IF: The TypeId doesn't correspond to a valid component
    pub selected_component_id: Property<TypeId>,
    /// Currently selected template node inside the current component
    /// INVALID_IF: TemplateNodeId doesn't correspond to an id in the component with id
    /// selected_component_id
    pub selected_template_node_ids: Property<Vec<TemplateNodeId>>,
    /// A copied subtree of nodes, used for internal copy/paste before
    /// we decide to write to clipboard
    /// INVALID_IF: no invalid states (that is the responsibility of the designer)
    pub clip_board: Property<SubTrees>,
    /// The currently open class name that is being edited in this component.
    /// Some(class) if editor is open, othewise None.
    /// INVALID_IF: this class doesn't exist for the component referenced by selected_component_id
    pub current_editor_class_name: Property<Option<String>>,

    //---------------glass------------------
    /// Size and color of the glass stage for the current view, this is the
    /// container that objects sized based on percentage in the view is sized
    /// from.
    /// INVALID_IF: no invalid states, but should probably not be very small
    /// or be of a very ugly color!
    pub stage: Property<StageInfo>,
    /// Glass to world/viewport to world camera transform.
    /// INVALID_IF: Composed of other transforms than uniform (positive) scaling
    /// and translation.
    pub glass_to_world_transform: Property<Transform2<Glass, World>>,
    /// Last known glass mouse position, useful to be able to query position
    /// from keystrokes.
    /// INVALID_IF: doesn't represent current mouse pos
    pub mouse_position: Property<Point2<Glass>>,
    /// Current tool state while in use (ie in the process of drawing a rect,
    /// moving an object, moving a control point, drawing a line)
    /// OBS: needs to be wrapped in Rc<RefCell since tool_behavior itself needs
    /// action_context which contains app_state
    /// INVALID_IF: no invalid states
    // WARNING: Don't directly call set on this. This results in the currently
    // active tool not being finished. Instead use SetToolBehaviour action
    pub tool_behavior: Property<Option<Rc<RefCell<dyn ToolBehavior>>>>,

    //---------------toolbar----------------
    /// Currently selected tool in the top toolbar
    /// INVALID_IF: no currently invalid states, note that tool_state is
    /// usually in some way derived from this selected tool state, but non-matching
    /// types should still be fine
    pub selected_tool: Property<Tool>,

    //---------------keyboard----------------
    /// Currently pressed keys, used mostly for querying modifier key state
    /// INVALID_IF: no invalid states
    pub modifiers: Property<HashSet<ModifierKey>>,

    //--------------settings-----------------
    /// Input mapper is responsible for keeping track of
    /// mapping from key to designer InputEvents, and allowing this
    /// to be configured
    /// INVALID_IF: no invalid states
    pub input_mapper: Property<InputMapper>,
}

impl Interpolatable for Tool {}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Tool {
    TodoTool,
    #[default]
    PointerPercent,
    PointerPixels,
    CreateComponent(ToolbarComponent),
    Paintbrush,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ToolbarComponent {
    #[default]
    Rectangle,
    Ellipse,
    Text,
    Stacker,
    Scroller,

    // form controls
    Checkbox,
    Textbox,
    Button,
    Slider,
    Dropdown,
    RadioSet,
}

impl Interpolatable for ProjectMode {}

#[derive(Default, Clone)]
pub enum ProjectMode {
    #[default]
    Edit,
    Playing,
}

#[pax]
#[engine_import_path("pax_engine")]
pub struct StageInfo {
    pub stage_width: u32,
    pub stage_height: u32,
    pub color: Color,
}
