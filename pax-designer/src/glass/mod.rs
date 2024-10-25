use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

use anyhow::anyhow;
use pax_engine::api::cursor::CursorStyle;
use pax_engine::api::Fill;
use pax_engine::api::*;
use pax_engine::math::{Point2, Transform2, TransformParts, Vector2};
use pax_engine::node_layout::TransformAndBounds;
use pax_engine::*;
use pax_manifest::{PaxType, TemplateNodeId, TypeId, UniqueTemplateNodeIdentifier};
use pax_std::*;
use serde::Deserialize;

use crate::controls::file_and_component_picker::SetLibraryState;
use crate::controls::settings::color_picker::close_color_pickers;
use crate::designer_node_type::DesignerNodeType;
use crate::granular_change_store::GranularManifestChangeStore;
use crate::model::action::orm::CreateComponent;
use crate::model::action::tool::SetToolBehaviour;
use crate::model::action::world::Translate;
use crate::model::action::world::{SelectMode, SelectNodes};
use crate::model::tools::ToolBehavior;
use crate::model::SelectionState;
use crate::model::{app_state::AppState, GlassNode};
use crate::utils::designer_cursor::{DesignerCursor, DesignerCursorType};
use crate::{message_log_display, model, SetStage, StageInfo};

use crate::math::coordinate_spaces::{self, World};
use crate::math::{self, AxisAlignedBox};
use crate::model::action::pointer::Pointer;
use crate::model::action::{Action, ActionContext, RaycastMode};
use crate::model::input::{Dir, ModifierKey};

pub mod control_point;
pub mod intent;
pub mod outline;
pub mod tool_editors;
pub mod wireframe_editor;

use self::intent::IntentDef;
pub use self::tool_editors::TextEdit;
use self::tool_editors::TextEditTool;
use crate::message_log_display::DesignerLogMsg;
use control_point::ControlPoint;
use intent::Intent;
use outline::PathOutline;
use wireframe_editor::WireframeEditor;


const DOUBLE_CLICK_MAX_MS: u64 = 400;

#[pax]
#[engine_import_path("pax_engine")]
#[file("glass/mod.pax")]
pub struct Glass {
    pub tool_visual: Property<ToolVisualizationState>,
    pub time_last_click: Property<u64>,

    // NOTE: This can't be replaced with subscribe until/when we support
    // subscribes inside subscribes
    pub _cursor_changed_listener: Property<bool>,
}

impl Glass {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        model::read_app_state_with_derived(|app_state, derived| {
            self.bind_tool_visual(ctx, app_state.tool_behavior.clone());
            self.bind_cursor(
                ctx,
                app_state.cursor.clone(),
                derived.selection_state.clone(),
            );
            self.bind_scroller_modification(ctx, derived.open_containers.clone());
        });
    }

    pub fn bind_tool_visual(
        &mut self,
        ctx: &NodeContext,
        tool_behavior: Property<Option<Rc<RefCell<dyn ToolBehavior>>>>,
    ) {
        let tool_visual = self.tool_visual.clone();
        ctx.subscribe(&[tool_behavior.untyped()], move || {
            tool_visual.replace_with(if let Some(tool_behavior) = tool_behavior.get() {
                tool_behavior.borrow().get_visual()
            } else {
                Property::default()
            });
        });
    }

    pub fn bind_cursor(
        &mut self,
        ctx: &NodeContext,
        cursor: Property<DesignerCursor>,
        selection_state: Property<SelectionState>,
    ) {
        let tool_visual = self.tool_visual.clone();
        let ctxp = ctx.clone();
        let cursor_changed_listener = self._cursor_changed_listener.clone();

        // This is slow. To speed it up, only do the generation + sending of
        // the cursor if either the type has changed, or rotation has changed by more than 5 degrees.
        let last_cursor = Rc::new(RefCell::new(DesignerCursor::default()));
        ctx.subscribe(
            &[
                tool_visual.untyped(),
                cursor.untyped(),
                selection_state.untyped(),
            ],
            move || {
                let cursor = cursor.get();
                let cursor_override = tool_visual.read(|tv| tv.cursor_override.clone());
                let total_selection_bounds = selection_state.read(|ss| ss.total_bounds.clone());
                let ctxpp = ctxp.clone();
                let deps = [total_selection_bounds.untyped()];

                let last_cursor = last_cursor.clone();
                cursor_changed_listener.replace_with(Property::computed(
                    move || {
                        let transform_parts: TransformParts =
                            total_selection_bounds.get().as_transform().into();
                        let mut cursor = if cursor_override == DesignerCursor::default() {
                            cursor
                        } else {
                            cursor_override
                        };
                        cursor.rotation_degrees += transform_parts.rotation.to_degrees();
                        let mut last_cursor = borrow_mut!(last_cursor);
                        // this is slow, only do it when significant changes have happened
                        if last_cursor.cursor_type != cursor.cursor_type
                            || (last_cursor.rotation_degrees - cursor.rotation_degrees).abs() > 5.0
                        {
                            ctxpp.set_cursor(cursor.to_cursor_style());
                            *last_cursor = cursor;
                        }
                        true
                    },
                    &deps,
                ));
            },
        );
    }

    // make scroller not clip if a child is selected
    // for now only scroller needs somewhat special behavior
    // might want to create more general double click framework at some point
    pub fn bind_scroller_modification(
        &mut self,
        ctx: &NodeContext,
        open_containers: Property<Vec<UniqueTemplateNodeIdentifier>>,
    ) {
        let manifest_changed_notifier = ctx
            .peek_local_store(
                |change_notification_store: &mut GranularManifestChangeStore| {
                    change_notification_store.get_manifest_any_change_notifier()
                },
            )
            .expect("should be inserted at designer root");
        let ctxp = ctx.clone();
        ctx.subscribe(
            &[manifest_changed_notifier, open_containers.untyped()],
            move || {
                let open = open_containers.get();

                for node in open {
                    if let Some(node_interface) =
                        ctxp.get_nodes_by_global_id(node.clone()).into_iter().next()
                    {
                        let open_container = open_containers.clone();
                        let deps = [open_container.untyped()];
                        // if this is a scroller, make it open if it's id is in the currently open container set
                        let _ = node_interface.with_properties(|scroller: &mut Scroller| {
                            scroller._clip_content.replace_with(Property::computed(
                                move || !open_container.get().contains(&node),
                                &deps,
                            ));
                        });
                    }
                }
            },
        )
    }

    pub fn on_pre_render(&mut self, _ctx: &NodeContext) {
        self._cursor_changed_listener.get();
    }

    pub fn context_menu(&mut self, _ctx: &NodeContext, args: Event<ContextMenu>) {
        args.prevent_default();
    }

    // NOTE: This is NOT triggered by engine - we are handling our own double click behavior
    // by checking time in self.handle_mouse_down
    pub fn handle_double_click(&mut self, ctx: &NodeContext, event: &Event<MouseDown>) {
        // TODO move to another file (need to figure out structure)
        let info = model::read_app_state(|app_state| {
            let selected_nodes = app_state.selected_template_node_ids.get();

            let Some(selected_node_id) = selected_nodes.last() else {
                return None;
            };
            let uid = UniqueTemplateNodeIdentifier::build(
                app_state.selected_component_id.get().clone(),
                selected_node_id.clone(),
            );
            let mut dt = borrow_mut!(ctx.designtime);
            let builder = dt.get_orm_mut().get_node_builder(
                uid.clone(),
                app_state.modifiers.get().contains(&ModifierKey::Control),
            )?;
            Some((builder.get_type_id(), uid))
        });
        if let Some((node_id, uid)) = info {
            let designer_node = DesignerNodeType::from_type_id(node_id);
            let metadata = designer_node.metadata(&borrow!(ctx.designtime).get_orm());

            match designer_node {
                _ if metadata.is_container => {
                    model::with_action_context(ctx, |ac| {
                        let hit = ac.raycast_glass(
                            ac.glass_transform().get()
                                * Point2::<Window>::new(event.mouse.x, event.mouse.y),
                            RaycastMode::DrillOne,
                            &[],
                        );
                        if let Some(hit) = hit {
                            if let Some(hit_type) = hit
                                .global_id()
                                .map(|id| id.get_containing_component_type_id())
                            {
                                // If the type id of the node hit during the drill hit is different than the current
                                // component, go into that component. If it's the same, this was a slot component
                                // and we want to drill into it.
                                if hit_type != ac.app_state.selected_component_id.get() {
                                    if let Err(_e) =
                                        SetEditingComponent(metadata.type_id.clone()).perform(ac)
                                    {
                                        message_log_display::log(DesignerLogMsg::message(format!("Cannot edit the component {} because it is not part of this codebase", &metadata.type_id.get_unique_identifier())));
                                    }
                                } else {
                                    if let Err(e) = (SelectNodes {
                                        ids: &[hit.global_id().unwrap().get_template_node_id()],
                                        mode: model::action::world::SelectMode::Dynamic,
                                    }
                                    .perform(ac))
                                    {
                                        log::warn!("failed to drill into container: {}", e);
                                    };
                                }
                            } else {
                                log::warn!("drill node had no type id");
                            }
                        }
                    });
                }
                DesignerNodeType::Text => {
                    model::perform_action(&TextEdit { uid }, ctx);
                }
                // Assume it's a component if it didn't have a custom impl for double click behavior
                DesignerNodeType::Component { .. } => {
                    model::perform_action(&SetEditingComponent(metadata.type_id), ctx)
                }
                _ => (),
            }
        }
    }

    pub fn handle_select_start(&mut self, ctx: &NodeContext, event: Event<SelectStart>) {
        // TODO change this to only do this if glass is "selected", as a proxy use if tool is in use
        // or not.
        if !model::with_action_context(ctx, |ac| {
            ac.app_state
                .tool_behavior
                .get()
                .is_some_and(|tool_behavior| {
                    let tool_type = (&*borrow!(tool_behavior)).type_id();
                    tool_type != std::any::TypeId::of::<TextEditTool>()
                })
        }) {
            event.prevent_default();
        }
    }

    pub fn handle_mouse_down(&mut self, ctx: &NodeContext, args: Event<MouseDown>) {
        if args.mouse.button == MouseButton::Left {
            let last_time = self.time_last_click.get();
            let curr_time = ctx.elapsed_time_millis() as u64;
            if (curr_time - last_time) < DOUBLE_CLICK_MAX_MS {
                self.handle_double_click(ctx, &args);
            }
            self.time_last_click.set(curr_time);
        }
        model::perform_action(
            &crate::model::action::pointer::MouseEntryPointAction {
                event: Pointer::Down,
                button: args.mouse.button.clone(),
                point: Point2::new(args.mouse.x, args.mouse.y),
            },
            ctx,
        );

        // TODO remove once input focus state handles this in the engine
        close_color_pickers();
    }

    pub fn handle_wheel(&mut self, ctx: &NodeContext, args: Event<Wheel>) {
        args.prevent_default();
        model::with_action_context(ctx, |ac| {
            let original = ac.app_state.glass_to_world_transform.get();
            if let Err(e) = (Translate {
                translation: Vector2::new(args.delta_x, args.delta_y),
                original_transform: original,
            }
            .perform(ac))
            {
                log::warn!("wheel action failed: {}", e);
            };
        });
    }

    pub fn handle_drop(&mut self, ctx: &NodeContext, event: Event<Drop>) {
        {
            let dt = borrow_mut!(ctx.designtime);
            if let Err(e) = dt.send_file_to_static_dir(&event.args.name, event.args.data) {
                log::warn!("failed to send file to server {}", e);
            } else {
                log::info!("sent file to server!!");
            };
        }
        let Some(parent) = ctx.get_userland_root_expanded_node() else {
            log::warn!("failed to handle image drop: couldn't get userland root");
            return;
        };
        model::with_action_context(ctx, |ac| {
            let parent = GlassNode::new(&parent, &ac.glass_transform());
            let cw = ac.glass_transform().get() * Point2::new(event.args.x, event.args.y);
            let v = Vector2::new(150.0, 150.0);
            let t = ac.transaction("creating object");
            let _ = t.run(|| {
                let uid = CreateComponent {
                    parent_id: &parent.id,
                    parent_index: pax_manifest::TreeIndexPosition::Top,
                    node_layout: Some(model::action::orm::NodeLayoutSettings::KeepScreenBounds {
                        node_transform_and_bounds: &TransformAndBounds {
                            transform: AxisAlignedBox::new(cw + v, cw - v).as_transform(),
                            bounds: (1.0, 1.0),
                        }
                        .as_pure_size(),
                        parent_transform_and_bounds: &parent.transform_and_bounds.get(),
                        node_decomposition_config: &Default::default(),
                    }),
                    designer_node_type: DesignerNodeType::Image,
                    builder_extra_commands: Some(&|builder| {
                        builder.set_property_from_typed(
                            "source",
                            Some(ImageSource::Url(format!("assets/{}", event.args.name))),
                        )
                    }),
                }
                .perform(ac)?;

                SelectNodes {
                    ids: &[uid.get_template_node_id()],
                    mode: SelectMode::DiscardOthers,
                }
                .perform(ac)
            });
        });
    }
}

pub struct SetEditingComponent(pub TypeId);

impl Action for SetEditingComponent {
    fn perform(&self, ctx: &mut ActionContext) -> anyhow::Result<()> {
        let type_id = &self.0;

        if !matches!(type_id.get_pax_type(), PaxType::Singleton { .. }) {
            return Err(anyhow!("tried to edit a non-component: {:?}", type_id));
        }

        let is_pax_std = type_id
            .import_path()
            .is_some_and(|p| p.starts_with("pax_std") || p.starts_with("pax_designer"));

        if is_pax_std {
            return Err(anyhow!(
                "tried to edit a non-userland comp: {:?}",
                type_id.import_path()
            ));
        }

        SetLibraryState { open: false }.perform(ctx)?;

        // TODO set stage defaults for opened component using "SetStage" action
        {
            let mut dt = borrow_mut!(ctx.engine_context.designtime);
            dt.set_userland_root_component_type_id(&self.0);
        }
        // reset other relevant app state
        ctx.app_state
            .selected_template_node_ids
            .update(|v| v.clear());
        ctx.app_state.selected_component_id.set(type_id.clone());
        Ok(())
    }
}

#[pax]
#[engine_import_path("pax_engine")]
#[custom(Default)]
pub struct ToolVisualizationState {
    /// rectangle drawing tool. Used during object creation
    /// and for multi-select
    pub rect_tool: RectTool,
    /// Highlight around an object when mouse is over
    pub object_outline: Vec<PathElement>,
    /// snap lines
    pub snap_lines: SnapInfo,
    /// only disabled when we need to interact with the nodes in the glass,
    /// for example when editing text
    pub event_blocker_active: bool,
    /// tool intent areas (not raycasted - drop behavior is handled separately by the tool itself)
    pub intent_areas: Vec<IntentDef>,
    /// the cursor to be shown
    pub cursor_override: DesignerCursor,
}

impl Default for ToolVisualizationState {
    fn default() -> Self {
        Self {
            rect_tool: Default::default(),
            object_outline: Default::default(),
            snap_lines: Default::default(),
            event_blocker_active: true,
            intent_areas: Default::default(),
            cursor_override: Default::default(),
        }
    }
}

#[pax]
#[engine_import_path("pax_engine")]
pub struct SnapInfo {
    pub vertical: Vec<SnapLine>,
    pub horizontal: Vec<SnapLine>,
    pub points: Vec<Vec<f64>>,
}

#[pax]
#[engine_import_path("pax_engine")]
pub struct SnapLine {
    pub line: f64,
    pub color: Color,
}

#[pax]
#[engine_import_path("pax_engine")]
#[custom(Default)]
pub struct RectTool {
    pub x: Size,
    pub y: Size,
    pub width: Size,
    pub height: Size,
    pub fill_color: Color,
    pub stroke_color: Color,
}

impl Default for RectTool {
    fn default() -> Self {
        RectTool {
            x: Size::ZERO(),
            y: Size::ZERO(),
            width: Size::ZERO(),
            height: Size::ZERO(),
            fill_color: Color::TRANSPARENT,
            stroke_color: Color::TRANSPARENT,
        }
    }
}
