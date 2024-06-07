pub mod action;
pub mod input;
pub mod tools;

use crate::glass;
use crate::glass::control_point::ControlPointBehaviour;
use crate::math::coordinate_spaces::World;
use crate::model::action::ActionContext;
use crate::model::input::RawInput;
use crate::USER_PROJ_ROOT_COMPONENT;
use crate::USER_PROJ_ROOT_IMPORT_PATH;
use action::Action;
use anyhow::anyhow;
use anyhow::Context;
use anyhow::Result;
use pax_designtime::DesigntimeManager;
use pax_engine::api::Color;
use pax_engine::api::Interpolatable;
use pax_engine::api::MouseButton;
use pax_engine::layout::LayoutProperties;
use pax_engine::layout::TransformAndBounds;
use pax_engine::math::Generic;
use pax_engine::math::{Transform2, Vector2};
use pax_engine::pax;
use pax_engine::NodeLocal;
use pax_engine::Property;
use pax_engine::{api::NodeContext, math::Point2};
use pax_manifest::TemplateNodeId;
use pax_manifest::TypeId;
use pax_manifest::UniqueTemplateNodeIdentifier;
use pax_runtime_api::borrow;
use std::cell::OnceCell;
use std::cell::RefCell;
use std::collections::HashSet;
use std::ops::ControlFlow;
use std::rc::Rc;

use crate::math::coordinate_spaces::{self, Glass};

use self::action::pointer::Pointer;
use self::action::pointer::PointerAction;
use self::action::UndoStack;
use self::input::{Dir, InputEvent, InputMapper};

const INITIALIZED: &'static str = "model should have been initialized";

// Needs to be changed if we use a multithreaded async runtime
thread_local! {
    static GLOBAL_STATE: RefCell<Option<GlobalDesignerState>> = RefCell::new(None);
}

pub struct GlobalDesignerState {
    pub undo_stack: UndoStack,
    pub app_state: AppState,
    pub derived_state: DerivedAppState,
}

pub fn init_model(ctx: &NodeContext) {
    let userland_project_root_type_id = TypeId::build_singleton(
        &format!(
            "{}::{}",
            USER_PROJ_ROOT_IMPORT_PATH, USER_PROJ_ROOT_COMPONENT
        ),
        None,
    );
    let app_state = AppState {
        selected_component_id: Property::new(userland_project_root_type_id.to_owned()),
        ..Default::default()
    };

    let ctx = ctx.clone();
    let comp_id = app_state.selected_component_id.clone();
    let node_ids = app_state.selected_template_node_ids.clone();
    // NOTE: ideally, the dependencies below are at some point removed and
    // replaced by a direct property dependency of expanded nodes in the engine
    // itself
    let manifest_ver = borrow!(ctx.designtime).get_manifest_version();

    let deps = [
        comp_id.untyped(),
        node_ids.untyped(),
        manifest_ver.untyped(),
    ];
    let selected_bounds = Property::computed(
        move || {
            let selected_bounds = with_action_context(&ctx, |ac| ac.selection_state());
            selected_bounds
        },
        &deps,
    );

    let derived_state = DerivedAppState { selected_bounds };

    GLOBAL_STATE.with_borrow_mut(|state| {
        *state = Some(GlobalDesignerState {
            undo_stack: UndoStack::default(),
            app_state,
            derived_state,
        })
    });
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
    /// The project mode (playing/editing)
    /// INVALID_IF: no invalid states
    pub project_mode: Property<ProjectMode>,
    /// The component currently being viewed and edited in the glass
    /// INVALID_IF: The TypeId doesn't correspond to a valid component
    pub selected_component_id: Property<TypeId>,
    /// Currently selected template node inside the current component
    /// INVALID_IF: TemplateNodeId doesn't correspond to an id in the component with id
    /// selected_component_id
    pub selected_template_node_ids: Property<Vec<TemplateNodeId>>,

    //---------------glass------------------
    /// Glass to world/viewport to world camera transform.
    /// INVALID_IF: Composed of other transforms than uniform (positive) scaling
    /// and translation.
    pub glass_to_world_transform: Property<Transform2<Glass, World>>,
    /// Last known glass mouse position, useful to be able to query positon
    /// from keystrokes.
    /// INVALID_IF: doesn't represent current mouse pos
    pub mouse_position: Property<Point2<Glass>>,
    /// Current tool state while in use (ie in the process of drawing a rect,
    /// moving an object, moving a control point, drawing a sline)
    /// OBS: needs to be wrapped in Rc<RefCell since tool_behaviour itself needs
    /// action_context which contains app_state
    /// INVALID_IF: no invalid states
    pub tool_behaviour: Property<Option<Rc<RefCell<dyn ToolBehaviour>>>>,
    /// Size and color of the glass stage for the current view, this is the
    /// container that objects sized based on percentage in the view is sized
    /// from.
    /// INVALID_IF: no invalid states, but should probably not be very small
    /// or be of a very ugly color!
    pub stage: Property<StageInfo>,

    //---------------toolbar----------------
    /// Currently selected tool in the top tool bar
    /// INVALID_IF: no currently invalid states, note that tool_state is
    /// usually in some way derived from this selected tool state, but non-matching
    /// types should still be fine
    pub selected_tool: Property<Tool>,

    //---------------keyboard----------------
    /// Currently pressed keys, used mostly for querying modifier key state
    /// INVALID_IF: no invalid states
    pub keys_pressed: Property<HashSet<InputEvent>>,

    //--------------settings-----------------
    /// Input mapper is responsible for keeping track of
    /// mapping from key to designer InputEvents, and allowing this
    /// to be configured
    /// INVALID_IF: no invalid states
    pub input_mapper: Property<InputMapper>,
}

pub fn read_app_state<T>(closure: impl FnOnce(&AppState) -> T) -> T {
    GLOBAL_STATE.with_borrow_mut(|model| closure(&model.as_ref().expect(INITIALIZED).app_state))
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
        } = model.as_mut().expect(INITIALIZED);
        func(&mut ActionContext {
            undo_stack,
            engine_context: ctx,
            app_state,
        })
    })
}

impl Interpolatable for SelectionState {}

pub struct Selection {}
#[derive(Clone, Default)]
pub struct SelectionState {
    total_bounds: Property<TransformAndBounds<Generic, Glass>>,
    items: Vec<SelectedItem>,
}

impl SelectionState {
    // Temporary way to get the signle selected object, or none.
    // This will be superseded once all operations on selectionstate
    // accept multiple objects
    pub fn get_single(&self) -> Option<&SelectedItem> {
        if self.items.len() == 1 {
            Some(&self.items[0])
        } else {
            None
        }
    }

    pub fn selected_count(&self) -> usize {
        self.items.len()
    }

    pub fn total_bounds(&self) -> Option<Property<TransformAndBounds<Generic, Glass>>> {
        if self.items.is_empty() {
            None
        } else if self.items.len() == 1 {
            let t_and_b = self.items[0].transform_and_bounds.clone();
            let deps = [t_and_b.untyped()];
            Some(Property::computed(
                move || {
                    let t_and_b = t_and_b.get();
                    let transform = t_and_b.transform.cast_spaces();
                    TransformAndBounds {
                        transform,
                        bounds: t_and_b.bounds,
                    }
                },
                &deps,
            ))
        } else {
            Some(self.total_bounds.clone())
        }
    }
}

#[derive(Default, Clone)]
pub struct SelectedItem {
    // unit rectangle to object bounds transform
    pub transform_and_bounds: Property<TransformAndBounds<NodeLocal, Glass>>,
    pub origin: Point2<Glass>,
    pub props: LayoutProperties,
    pub id: UniqueTemplateNodeIdentifier,
}

// This represents values that can be deterministically produced from the app
// state and the projects manifest
pub struct DerivedAppState {
    pub selected_bounds: Property<SelectionState>,
}

pub fn read_app_state_with_derived<V>(closure: impl FnOnce(&AppState, &DerivedAppState) -> V) -> V {
    GLOBAL_STATE.with_borrow(|model| {
        let model = model.as_ref().expect(INITIALIZED);
        closure(&model.app_state, &model.derived_state)
    })
}

pub fn perform_action(action: impl Action, ctx: &NodeContext) {
    if let Err(e) = with_action_context(ctx, |ac| ac.execute(action)) {
        pax_engine::log::warn!("action failed: {:?}", e);
    }
}

pub fn process_keyboard_input(ctx: &NodeContext, dir: Dir, input: String) {
    // useful! keeping around for now
    // pax_engine::log::info!("key {:?}: {}", dir, input);
    let action = GLOBAL_STATE.with_borrow_mut(|model| -> anyhow::Result<Option<Box<dyn Action>>> {
        let raw_input = RawInput::try_from(input)?;
        let AppState {
            ref mut input_mapper,
            ref mut keys_pressed,
            ..
        } = &mut model.as_mut().expect(INITIALIZED).app_state;

        let input_mapper = input_mapper.get();
        let event = input_mapper
            .to_event(raw_input)
            .with_context(|| "no mapped input")?;
        keys_pressed.update(|keys_pressed| {
            match dir {
                Dir::Down => keys_pressed.insert(event.clone()),
                Dir::Up => keys_pressed.remove(event),
            };
        });
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
        let tool_behaviour = ctx.app_state.tool_behaviour.clone();
        tool_behaviour.update(|tool_behaviour| {
            if let Some(tool) = tool_behaviour {
                let mut tool = tool.borrow_mut();
                tool.pointer_move(ctx.app_state.mouse_position.get(), ctx);
            }
        });
    });
}

impl Interpolatable for Tool {}
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
    Text,
}

pub trait ToolBehaviour {
    fn pointer_down(&mut self, point: Point2<Glass>, ctx: &mut ActionContext) -> ControlFlow<()>;
    fn pointer_move(&mut self, point: Point2<Glass>, ctx: &mut ActionContext) -> ControlFlow<()>;
    fn pointer_up(&mut self, point: Point2<Glass>, ctx: &mut ActionContext) -> ControlFlow<()>;
    fn keyboard(&mut self, event: InputEvent, dir: Dir, ctx: &mut ActionContext)
        -> ControlFlow<()>;
    fn visualize(&self, glass: &mut crate::glass::Glass);
}

impl Interpolatable for ProjectMode {}

#[derive(Default, Clone)]
pub enum ProjectMode {
    Edit,
    #[default]
    Playing,
}

#[derive(Clone)]
pub enum ControlPointPos {
    First,
    Middle,
    Last,
}

#[pax]
pub struct StageInfo {
    pub width: u32,
    pub height: u32,
    pub color: Color,
}
