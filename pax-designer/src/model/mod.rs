use crate::glass;
use crate::glass::control_point::ControlPointBehavior;
use crate::glass::ToolVisualizationState;
use crate::granular_change_store::GranularManifestChangeStore;
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
use pax_engine::api::borrow_mut;
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
use pax_engine::PaxValue;
use pax_engine::Property;
use pax_engine::{api::borrow, api::NodeContext, math::Point2};
use std::any::Any;
use std::cell::OnceCell;
use std::cell::RefCell;
use std::collections::BTreeSet;
use std::collections::HashMap;
use std::collections::HashSet;
use std::ops::ControlFlow;
use std::rc::Rc;

use crate::math::coordinate_spaces::{self, Glass};
pub use selection_state::*;

use self::action::pointer::MouseEntryPointAction;
use self::action::pointer::Pointer;
use self::action::Transaction;
use self::action::UndoRedoStack;
pub use self::app_state::AppState;
use self::app_state::StageInfo;
pub use self::derived_state::DerivedAppState;
use self::input::ModifierKey;
use self::input::{Dir, InputEvent};
use self::tools::ToolBehavior;

pub mod action;
pub mod app_state;
pub mod derived_state;
pub mod input;
mod selection_state;
pub mod tools;

const INITIALIZED: &'static str = "model should have been initialized";

// Needs to be changed if we use a multithreaded async runtime
thread_local! {
    static MODEL: RefCell<Option<Model>> = RefCell::new(None);
}

pub struct Model {
    pub undo_stack: Rc<UndoRedoStack>,
    pub app_state: app_state::AppState,
    pub derived_state: derived_state::DerivedAppState,
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
        let manifest_changed_notifier = ctx
            .peek_local_store(
                |change_notification_store: &mut GranularManifestChangeStore| {
                    change_notification_store.get_manifest_any_change_notifier()
                },
            )
            .expect("should be inserted at designer root");
        let ctx_cp = ctx.clone();

        let deps = [
            comp_id.untyped(),
            node_ids.untyped(),
            // Needs to listen to manifest changes to re-get the correct engine
            // nodes after a reload
            manifest_changed_notifier,
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

// TODO remove this? If you want an action context, use a new action
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
