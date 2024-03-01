pub mod action;
pub mod input;
pub mod math;
pub mod tools;

use crate::glass;
use crate::glass::control_point::ControlPointBehaviour;
use crate::math::AxisAlignedBox;
use crate::model::action::ActionContext;
use crate::model::input::RawInput;
use action::Action;
use anyhow::anyhow;
use anyhow::Context;
use anyhow::Result;
use math::coordinate_spaces::World;
use pax_designtime::DesigntimeManager;
use pax_engine::api::MouseButton;
use pax_engine::math::{Transform2, Vector2};
use pax_engine::{api::NodeContext, math::Point2, rendering::TransformAndBounds};
use pax_manifest::TemplateNodeId;
use pax_manifest::TypeId;
use pax_std::types::Color;
use std::cell::RefCell;
use std::collections::HashSet;
use std::ops::ControlFlow;
use std::rc::Rc;

use math::coordinate_spaces::Glass;

use self::action::pointer::Pointer;
use self::action::pointer::PointerAction;
use self::input::{Dir, InputEvent, InputMapper};
use self::math::coordinate_spaces;

// Needs to be changed if we use a multithreaded async runtime
thread_local!(
    static GLOBAL_STATE: RefCell<GlobalDesignerState> = RefCell::new(GlobalDesignerState::new());
);

#[derive(Default)]
pub struct GlobalDesignerState {
    pub undo_stack: action::UndoStack,
    pub app_state: AppState,
}

impl GlobalDesignerState {
    fn new() -> Self {
        Self {
            app_state: AppState {
                selected_component_id: TypeId::build_singleton(
                    "pax_designer::pax_reexports::designer_project::Example",
                    None,
                )
                .to_owned(),
                ..Default::default()
            },
            ..Default::default()
        }
    }
}

/// Represents the global source-of-truth for the desinger.
/// Invalid if any of the bellow :INVALID_IF: statements hold true.
/// Effort has been put in to expose as few invalid states as possible.
/// NOTE: Only add a new field to this struct if:
/// 1. There is no way of computing the sought after value from a combination
/// of AppState and Manifest information (if this is true, make it part of the
/// DerivedAppState struct)
/// 2. An effort has been made to reduce the number of invalid states:
/// use an enum instead of an usize if there exists a fixed set of of options,
/// and try to drill down and add state in inner AppState variables if it makes
/// sense (for example try to slot things like state of the currently used tool
/// into the tool_state variable, since no two toolstates can exist at the same time)
#[derive(Default)]
#[deny(missing_docs)]
pub struct AppState {
    //---------------global-----------------
    /// The component currently being viewed and edited in the glass
    /// INVALID_IF: String doesn't correspond to a component path
    pub selected_component_id: TypeId,
    /// Currently selected template node inside the current component
    /// INVALID_IF: usize doesn't correspond to an id in the component with id
    /// selected_component_id
    pub selected_template_node_id: Option<TemplateNodeId>,

    //---------------glass------------------
    /// Glass to world/viewport to world camera transform.
    /// INVALID_IF: Composed of other transforms than uniform (positive) scaling
    /// and translation.
    pub glass_to_world_transform: Transform2<Glass, World>,
    /// Last known glass mouse position, useful to be able to query positon
    /// from keystrokes.
    /// INVALID_IF: doesn't represent current mouse pos
    pub mouse_position: Point2<Glass>,
    /// Current tool state while in use (ie in the process of drawing a rect,
    /// moving an object, moving a control point, drawing a sline)
    /// OBS: needs to be wrapped in Rc<RefCell since tool_behaviour itself needs
    /// action_context which contains app_state
    /// INVALID_IF: no invalid states
    pub tool_behaviour: Rc<RefCell<Option<Box<dyn ToolBehaviour>>>>,

    //---------------toolbar----------------
    /// Currently selected tool in the top tool bar
    /// INVALID_IF: no currently invalid states, note that tool_state is
    /// usually in some way derived from this selected tool state, but non-matching
    /// types should still be fine
    pub selected_tool: Tool,

    //---------------keyboard----------------
    /// Currently pressed keys, used mostly for querying modifier key state
    /// INVALID_IF: no invalid states
    pub keys_pressed: HashSet<InputEvent>,

    //--------------settings-----------------
    /// Input mapper is responsible for keeping track of
    /// mapping from key to designer InputEvents, and allowing this
    /// to be configured
    /// INVALID_IF: no invalid states
    pub input_mapper: InputMapper,
}

pub fn read_app_state(closure: impl FnOnce(&AppState)) {
    GLOBAL_STATE.with_borrow_mut(|model| {
        closure(&model.app_state);
    });
}

pub fn with_action_context<R: 'static>(
    ctx: &NodeContext,
    func: impl FnOnce(&mut ActionContext) -> R,
) -> R {
    GLOBAL_STATE.with_borrow_mut(|model| {
        let GlobalDesignerState {
            ref mut undo_stack,
            ref mut app_state,
            ..
        } = *model;
        func(&mut ActionContext {
            undo_stack,
            engine_context: ctx,
            app_state,
        })
    })
}

// This represents values that can be deterministically produced from the app
// state and the projects manifest
pub struct DerivedAppState {
    pub selected_bounds: Option<(AxisAlignedBox, Point2<Glass>)>,
}

pub fn read_app_state_with_derived(
    ctx: &NodeContext,
    closure: impl FnOnce(&AppState, &DerivedAppState),
) {
    let selected_bounds = with_action_context(ctx, |ac| ac.selected_bounds());
    GLOBAL_STATE.with_borrow_mut(|model| {
        closure(&model.app_state, &DerivedAppState { selected_bounds });
    });
}

pub fn perform_action(action: impl Action, ctx: &NodeContext) {
    if let Err(e) = with_action_context(ctx, |ac| ac.execute(action)) {
        pax_engine::log::warn!("action failed: {:?}", e);
    }
}

pub fn process_keyboard_input(ctx: &NodeContext, dir: Dir, input: String) {
    // useful! keeping around for now
    // pax_engine::log::debug!("key {:?}: {}", dir, input);
    let action = GLOBAL_STATE.with_borrow_mut(|model| -> anyhow::Result<Option<Box<dyn Action>>> {
        let raw_input = RawInput::try_from(input)?;
        let AppState {
            ref mut input_mapper,
            ref mut keys_pressed,
            ..
        } = &mut model.app_state;

        let event = input_mapper
            .to_event(raw_input)
            .with_context(|| "no mapped input")?;
        match dir {
            Dir::Down => keys_pressed.insert(event.clone()),
            Dir::Up => keys_pressed.remove(event),
        };
        let action = input_mapper.to_action(event, dir);
        Ok(action)
    });
    match action {
        Ok(Some(action)) => {
            perform_action(action, ctx);
        }
        Ok(None) => (),
        Err(e) => pax_engine::log::warn!("couldn't keyboard mapping: {:?}", e),
    }
    // Trigger tool move in case the current tool
    // changes behaviour when for example Alt is pressed.
    // No-op if no tool is in use
    with_action_context(ctx, |ctx| {
        let tool_behaviour = Rc::clone(&ctx.app_state.tool_behaviour);
        let mut tool_behaviour = tool_behaviour.borrow_mut();
        if let Some(tool) = tool_behaviour.as_mut() {
            tool.pointer_move(ctx.app_state.mouse_position, ctx);
        }
    });
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Tool {
    TodoTool,
    #[default]
    Pointer,
    CreateComponent(Component),
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Component {
    #[default]
    Rectangle,
    Ellipse,
}

pub trait ToolBehaviour {
    fn pointer_down(&mut self, point: Point2<Glass>, ctx: &mut ActionContext) -> ControlFlow<()>;
    fn pointer_move(&mut self, point: Point2<Glass>, ctx: &mut ActionContext) -> ControlFlow<()>;
    fn pointer_up(&mut self, point: Point2<Glass>, ctx: &mut ActionContext) -> ControlFlow<()>;
    fn keyboard(&mut self, event: InputEvent, dir: Dir, ctx: &mut ActionContext)
        -> ControlFlow<()>;
    fn visualize(&self, glass: &mut glass::Glass);
}

#[derive(Clone)]
pub enum ControlPointPos {
    First,
    Middle,
    Last,
}
