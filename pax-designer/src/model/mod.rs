pub mod action;
pub mod input;
pub mod tools;

use crate::glass;
use crate::glass::control_point::ControlPointBehavior;
use crate::glass::ToolVisualizationState;
use crate::math::coordinate_spaces::SelectionSpace;
use crate::math::coordinate_spaces::World;
use crate::math::SizeUnit;
use crate::model::action::ActionContext;
use crate::model::input::RawInput;
use crate::DESIGNER_GLASS_ID;
use action::Action;
use anyhow::anyhow;
use anyhow::Context;
use anyhow::Result;
use pax_designtime::orm::SubTrees;
use pax_designtime::DesigntimeManager;
use pax_engine::api::Color;
use pax_engine::api::Interpolatable;
use pax_engine::api::MouseButton;
use pax_engine::api::Window;
use pax_engine::layout::LayoutProperties;
use pax_engine::layout::TransformAndBounds;
use pax_engine::log;
use pax_engine::math::Generic;
use pax_engine::math::{Transform2, Vector2};
use pax_engine::pax;
use pax_engine::pax_manifest::PropertyDefinition;
use pax_engine::pax_manifest::TemplateNodeId;
use pax_engine::pax_manifest::TypeId;
use pax_engine::pax_manifest::UniqueTemplateNodeIdentifier;
use pax_engine::pax_manifest::Unit;
use pax_engine::pax_manifest::ValueDefinition;
use pax_engine::NodeInterface;
use pax_engine::NodeLocal;
use pax_engine::Property;
use pax_engine::{api::borrow, api::NodeContext, math::Point2};
use std::any::Any;
use std::cell::OnceCell;
use std::cell::RefCell;
use std::collections::BTreeSet;
use std::collections::HashSet;
use std::ops::ControlFlow;
use std::rc::Rc;

use crate::math::coordinate_spaces::{self, Glass};
mod selection_state;
pub use selection_state::*;

use self::action::pointer::MouseEntryPointAction;
use self::action::pointer::Pointer;
use self::action::UndoRedoStack;
use self::input::{Dir, InputEvent, InputMapper};

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
    pub keys_pressed: Property<HashSet<InputEvent>>,

    //--------------settings-----------------
    /// Input mapper is responsible for keeping track of
    /// mapping from key to designer InputEvents, and allowing this
    /// to be configured
    /// INVALID_IF: no invalid states
    pub input_mapper: Property<InputMapper>,
}

// This represents values that can be deterministically produced from the app
// state and the projects manifest
pub struct DerivedAppState {
    pub to_glass_transform: Property<Property<Transform2<Window, Glass>>>,
    pub selected_nodes: Property<Vec<(UniqueTemplateNodeIdentifier, NodeInterface)>>,
    pub selection_state: Property<SelectionState>,
    pub open_container: Property<UniqueTemplateNodeIdentifier>,
}

const INITIALIZED: &'static str = "model should have been initialized";

// Needs to be changed if we use a multithreaded async runtime
thread_local! {
    static MODEL: RefCell<Option<Model>> = RefCell::new(None);
}

pub struct Model {
    pub undo_stack: Rc<UndoRedoStack>,
    pub app_state: AppState,
    pub derived_state: DerivedAppState,
}

impl Model {
    pub fn init(ctx: &NodeContext) {
        let main_component_id = (*ctx.designtime).ref_cell.borrow_mut().get_manifest().main_component_type_id.clone();
        let app_state = Self::create_initial_app_state(main_component_id);
        let derived_state = Self::create_derived_state(ctx, &app_state);

        MODEL.with_borrow_mut(|state| {
            *state = Some(Model {
                undo_stack: Rc::new(UndoRedoStack::default()),
                app_state,
                derived_state,
            })
        });
    }

    fn create_initial_app_state(main_component_type_id: TypeId) -> AppState {

        AppState {
            selected_component_id: Property::new(main_component_type_id),
            stage: Property::new(StageInfo {
                width: 1380,
                height: 786,
                color: Color::WHITE,
            }),
            ..Default::default()
        }
    }

    fn create_derived_state(ctx: &NodeContext, app_state: &AppState) -> DerivedAppState {
        let selected_nodes = Self::derive_selected_nodes(ctx, app_state);
        let to_glass_transform = Self::derive_to_glass_transform(ctx);
        let selection_state =
            Self::derive_selection_state(selected_nodes.clone(), to_glass_transform.clone());
        let open_containers = Self::derive_open_container(ctx, app_state);

        DerivedAppState {
            to_glass_transform,
            selection_state,
            open_container: open_containers,
            selected_nodes,
        }
    }

    fn derive_selected_nodes(
        ctx: &NodeContext,
        app_state: &AppState,
    ) -> Property<Vec<(UniqueTemplateNodeIdentifier, NodeInterface)>> {
        let comp_id = app_state.selected_component_id.clone();
        let node_ids = app_state.selected_template_node_ids.clone();
        let manifest_ver = borrow!(ctx.designtime).get_manifest_version();
        let ctx_cp = ctx.clone();

        let deps = [
            comp_id.untyped(),
            node_ids.untyped(),
            manifest_ver.untyped(),
        ];

        Property::computed(
            move || {
                let type_id = comp_id.get();
                let mut nodes = vec![];
                let mut selected_ids = node_ids.get();
                let mut discarded = false;
                selected_ids.retain(|id| {
                    let unid = UniqueTemplateNodeIdentifier::build(type_id.clone(), id.clone());
                    let Some(node) = ctx_cp
                        .get_nodes_by_global_id(unid.clone())
                        .into_iter()
                        .max()
                    else {
                        discarded = true;
                        return false;
                    };
                    nodes.push((unid, node));
                    true
                });
                if discarded {
                    node_ids.set(selected_ids);
                }
                nodes
            },
            &deps,
        )
    }

    fn derive_to_glass_transform(
        ctx: &NodeContext,
    ) -> Property<Property<Transform2<Window, Glass>>> {
        let ctx_cp = ctx.clone();
        Property::computed(
            move || {
                let container = ctx_cp.get_nodes_by_id(DESIGNER_GLASS_ID);
                if let Some(userland_proj) = container.first() {
                    let t_and_b = userland_proj.transform_and_bounds();
                    let deps = [t_and_b.untyped()];
                    Property::computed(
                        move || {
                            t_and_b
                                .get()
                                .transform
                                .inverse()
                                .cast_spaces::<Window, Glass>()
                        },
                        &deps,
                    )
                } else {
                    panic!("no userland project")
                }
            },
            &[],
        )
    }

    fn derive_selection_state(
        selected_nodes: Property<Vec<(UniqueTemplateNodeIdentifier, NodeInterface)>>,
        to_glass_transform: Property<Property<Transform2<Window, Glass>>>,
    ) -> Property<SelectionState> {
        let deps = [selected_nodes.untyped(), to_glass_transform.untyped()];
        Property::computed(
            move || SelectionState::new(selected_nodes.get(), to_glass_transform.get()),
            &deps,
        )
    }

    fn derive_open_container(
        ctx: &NodeContext,
        app_state: &AppState,
    ) -> Property<UniqueTemplateNodeIdentifier> {
        let selected_comp = app_state.selected_component_id.clone();
        let node_ids = app_state.selected_template_node_ids.clone();
        let ctx_cp = ctx.clone();

        let deps = [selected_comp.untyped(), node_ids.untyped()];
        Property::computed(
            move || {
                let mut containers = HashSet::new();
                for n in node_ids.get() {
                    let uid = UniqueTemplateNodeIdentifier::build(selected_comp.get(), n);
                    let interface = ctx_cp.get_nodes_by_global_id(uid);
                    if let Some(parent_uid) = interface
                        .first()
                        .and_then(|v| v.template_parent().unwrap().global_id())
                    {
                        containers.insert(parent_uid);
                    }
                }
                if containers.len() == 1 {
                    containers.into_iter().next().unwrap()
                } else {
                    let root = ctx_cp
                        .get_userland_root_expanded_node();
                    root.global_id().unwrap()
                }
            },
            &deps,
        )
    }
}

pub fn read_app_state<T>(closure: impl FnOnce(&AppState) -> T) -> T {
    MODEL.with_borrow_mut(|model| closure(&model.as_ref().expect(INITIALIZED).app_state))
}

pub fn with_action_context<R: 'static>(
    ctx: &NodeContext,
    func: impl FnOnce(&mut ActionContext) -> R,
) -> R {
    MODEL.with_borrow_mut(|model| {
        let Model {
            ref undo_stack,
            ref mut app_state,
            ref mut derived_state,
            ..
        } = model.as_mut().expect(INITIALIZED);
        func(&mut ActionContext {
            undo_stack,
            engine_context: ctx,
            app_state,
            derived_state,
        })
    })
}

pub fn read_app_state_with_derived<V>(closure: impl FnOnce(&AppState, &DerivedAppState) -> V) -> V {
    MODEL.with_borrow(|model| {
        let model = model.as_ref().expect(INITIALIZED);
        closure(&model.app_state, &model.derived_state)
    })
}

pub fn perform_action(action: &dyn Action, ctx: &NodeContext) {
    if let Err(e) = with_action_context(ctx, |ac| action.perform(ac)) {
        pax_engine::log::warn!("action failed: {:?}", e);
    }
}

pub fn process_keyboard_input(ctx: &NodeContext, dir: Dir, input: String) {
    // useful! keeping around for now
    // pax_engine::log::info!("key {:?}: {}", dir, input);
    let action = MODEL.with_borrow_mut(|model| -> anyhow::Result<Option<Box<dyn Action>>> {
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
            perform_action(action.as_ref(), ctx);
        }
        Ok(None) => (),
        Err(e) => pax_engine::log::warn!("couldn't keyboard mapping: {:?}", e),
    }
    // Trigger tool move in case the current tool
    // changes behavior when for example Alt is pressed.
    // No-op if no tool is in use
    with_action_context(ctx, |ctx| {
        let tool_behavior = ctx.app_state.tool_behavior.clone();
        tool_behavior.update(|tool_behavior| {
            if let Some(tool) = tool_behavior {
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
    PointerPercent,
    PointerPixels,
    CreateComponent(Component),
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Component {
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

pub trait ToolBehavior {
    fn pointer_down(&mut self, point: Point2<Glass>, ctx: &mut ActionContext) -> ControlFlow<()>;
    fn pointer_move(&mut self, point: Point2<Glass>, ctx: &mut ActionContext) -> ControlFlow<()>;
    fn pointer_up(&mut self, point: Point2<Glass>, ctx: &mut ActionContext) -> ControlFlow<()>;
    fn keyboard(&mut self, event: InputEvent, dir: Dir, ctx: &mut ActionContext)
        -> ControlFlow<()>;
    fn get_visual(&self) -> Property<ToolVisualizationState>;
}

impl Interpolatable for ProjectMode {}

#[derive(Default, Clone)]
pub enum ProjectMode {
    #[default]
    Edit,
    Playing,
}

#[pax]
#[engine_import_prefix("pax_engine")]
pub struct StageInfo {
    pub width: u32,
    pub height: u32,
    pub color: Color,
}
