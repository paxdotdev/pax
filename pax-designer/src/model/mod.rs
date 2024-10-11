pub mod action;
pub mod input;
pub mod tools;

use crate::glass;
use crate::glass::control_point::ControlPointBehavior;
use crate::glass::ToolVisualizationState;
use crate::math::coordinate_spaces::SelectionSpace;
use crate::math::coordinate_spaces::World;
use crate::math::SizeUnit;
use crate::model;
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
use pax_engine::log;
use pax_engine::math::Generic;
use pax_engine::math::{Transform2, Vector2};
use pax_engine::node_layout::LayoutProperties;
use pax_engine::node_layout::TransformAndBounds;
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
use self::action::Transaction;
use self::action::UndoRedoStack;
use self::input::ModifierKey;
use self::input::{Dir, InputEvent, InputMapper};

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

// This represents values that can be deterministically produced from the app
// state and the projects manifest
pub struct DerivedAppState {
    pub to_glass_transform: Property<Property<Transform2<Window, Glass>>>,
    pub selected_nodes: Property<Vec<(UniqueTemplateNodeIdentifier, NodeInterface)>>,
    pub selection_state: Property<SelectionState>,
    /// The currently open containers, example: the parent group of the rectangle currently selected, and the scroller this group is inside
    pub open_containers: Property<Vec<UniqueTemplateNodeIdentifier>>,
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
        let main_component_id = (*ctx.designtime)
            .borrow_mut()
            .get_manifest()
            .main_component_type_id
            .clone();
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
                stage_width: 1380,
                stage_height: 786,
                color: Color::WHITE,
            }),
            ..Default::default()
        }
    }

    fn create_derived_state(ctx: &NodeContext, app_state: &AppState) -> DerivedAppState {
        let selected_nodes = Self::derive_selected_nodes(ctx, app_state);
        // WARNING: if you change this, also change the glass position in the src/lib.pax file
        // changed to make sure this value is correctly initialized even if we start in play mode
        let to_glass_transform = Property::new(Property::new(Transform2::translate(Vector2::new(
            -240.0, -60.0,
        )))); // Self::derive_to_glass_transform(ctx);
        let selection_state =
            Self::derive_selection_state(selected_nodes.clone(), to_glass_transform.clone());
        let open_containers = Self::derive_open_container(ctx, app_state);

        DerivedAppState {
            to_glass_transform,
            selection_state,
            open_containers,
            selected_nodes,
        }
    }

    fn derive_selected_nodes(
        ctx: &NodeContext,
        app_state: &AppState,
    ) -> Property<Vec<(UniqueTemplateNodeIdentifier, NodeInterface)>> {
        let comp_id = app_state.selected_component_id.clone();
        let node_ids = app_state.selected_template_node_ids.clone();
        let manifest_ver = borrow!(ctx.designtime).get_last_rendered_manifest_version();
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

    /// TODO use this again to get glass location dynamically if needed
    #[allow(unused)]
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
                    log::error!("designer glass node not found");
                    Property::default()
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
    ) -> Property<Vec<UniqueTemplateNodeIdentifier>> {
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
                    let mut direct_parent = containers.into_iter().next().unwrap();
                    let mut containers = vec![direct_parent.clone()];
                    while let Some(next_parent) = ctx_cp
                        .get_nodes_by_global_id(direct_parent.clone())
                        .into_iter()
                        .next()
                        .and_then(|v| v.template_parent())
                    {
                        containers.push(next_parent.global_id().unwrap());
                        direct_parent = next_parent.global_id().unwrap();
                    }
                    containers
                } else {
                    let root = ctx_cp.get_userland_root_expanded_node();
                    vec![root.and_then(|n| n.global_id()).unwrap()]
                }
            },
            &deps,
        )
    }
}

pub fn read_app_state<T>(closure: impl FnOnce(&AppState) -> T) -> T {
    MODEL.with_borrow(|model| closure(&model.as_ref().expect(INITIALIZED).app_state))
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
        func(&mut ActionContext::new(
            ctx,
            app_state,
            derived_state,
            undo_stack,
        ))
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
            ref input_mapper,
            ref modifiers,
            ..
        } = model.as_ref().expect(INITIALIZED).app_state;

        let input_mapper = input_mapper.get();
        let event = input_mapper
            .to_event(raw_input, dir, modifiers.clone())
            .with_context(|| "no mapped input")?;
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
        let tool_behavior = tool_behavior.get();
        if let Some(tool) = tool_behavior {
            let mut tool = tool.borrow_mut();
            tool.pointer_move(ctx.app_state.mouse_position.get(), ctx);
        }
    });
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

pub trait ToolBehavior {
    fn pointer_down(&mut self, point: Point2<Glass>, ctx: &mut ActionContext) -> ControlFlow<()>;
    fn pointer_move(&mut self, point: Point2<Glass>, ctx: &mut ActionContext) -> ControlFlow<()>;
    fn pointer_up(&mut self, point: Point2<Glass>, ctx: &mut ActionContext) -> ControlFlow<()>;
    /// called before this tools get's replaced by another one: for example to commit text
    /// when the TextEdit tool get's replaced.
    fn finish(&mut self, ctx: &mut ActionContext) -> Result<()>;
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
#[engine_import_path("pax_engine")]
pub struct StageInfo {
    pub stage_width: u32,
    pub stage_height: u32,
    pub color: Color,
}
