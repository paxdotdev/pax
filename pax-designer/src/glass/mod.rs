use anyhow::anyhow;
use pax_engine::api::Fill;
use pax_engine::api::*;
use pax_engine::math::{Point2, Transform2, Vector2};
use pax_engine::node_layout::TransformAndBounds;
use pax_engine::*;
use pax_manifest::{PaxType, TemplateNodeId, TypeId, UniqueTemplateNodeIdentifier};
use pax_std::*;
use serde::Deserialize;

use crate::controls::file_and_component_picker::SetLibraryState;
use crate::designer_node_type::DesignerNodeType;
use crate::model::action::orm::CreateComponent;
use crate::model::action::tool::SetToolBehaviour;
use crate::model::action::world::Translate;
use crate::model::action::world::{SelectMode, SelectNodes};
use crate::model::{AppState, GlassNode};
use crate::{message_log_display, model, SetStage, StageInfo};

use crate::math::coordinate_spaces::{self, World};
use crate::math::{self, AxisAlignedBox};
use crate::model::action::pointer::Pointer;
use crate::model::action::{Action, ActionContext, RaycastMode};
use crate::model::input::{Dir, ModifierKey};

pub mod control_point;
pub mod outline;
pub mod tool_editors;
pub mod wireframe_editor;

pub use self::tool_editors::TextEdit;
use crate::message_log_display::DesignerLogMsg;
use control_point::ControlPoint;
use outline::PathOutline;
use wireframe_editor::WireframeEditor;

#[pax]
#[engine_import_path("pax_engine")]
#[file("glass/mod.pax")]
pub struct Glass {
    pub tool_visual: Property<ToolVisualizationState>,
    pub on_tool_change: Property<bool>,
    // NOTE: these can be removed when for loops support nested calls:
    // self.tool_visual.snap_lines.vertical/self.tool_visual.snap_lines.horizontal
    pub tool_visual_snap_lines_vertical: Property<Vec<SnapLine>>,
    pub tool_visual_snap_lines_horizontal: Property<Vec<SnapLine>>,
    pub tool_visual_snap_lines_points: Property<Vec<Vec<f64>>>,
    pub tool_visual_event_blocker_active: Property<bool>,

    // used to make scroller containers open if manifest version changed
    pub scroller_manifest_version_listener: Property<bool>,
}

impl Glass {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        let tool_behavior = model::read_app_state(|app_state| app_state.tool_behavior.clone());
        let deps = [tool_behavior.untyped()];
        let tool_visual = self.tool_visual.clone();
        let (mouse_pos, world_transform) = model::read_app_state(|app_state| {
            (
                app_state.mouse_position.clone(),
                app_state.glass_to_world_transform.clone(),
            )
        });
        let tool_visual_snap_lines_vertical = self.tool_visual_snap_lines_vertical.clone();
        let tool_visual_snap_lines_horizontal = self.tool_visual_snap_lines_horizontal.clone();
        let tool_visual_snap_lines_points = self.tool_visual_snap_lines_points.clone();
        let tool_visual_event_blocker_active = self.tool_visual_event_blocker_active.clone();
        let ctxp = ctx.clone();
        self.on_tool_change.replace_with(Property::computed(
            move || {
                tool_visual.replace_with(if let Some(tool_behavior) = tool_behavior.get() {
                    let tool_visual_state = tool_behavior.borrow_mut().get_visual();
                    // NOTE: these can be removed when for loops support nested calls:
                    // self.tool_visual.snap_lines.vertical/self.tool_visual.snap_lines.horizontal
                    let tv = tool_visual_state.clone();
                    let deps = [tv.untyped()];
                    tool_visual_snap_lines_vertical.replace_with(Property::computed(
                        move || tv.get().snap_lines.vertical,
                        &deps,
                    ));
                    let tv = tool_visual_state.clone();
                    tool_visual_snap_lines_horizontal.replace_with(Property::computed(
                        move || tv.get().snap_lines.horizontal,
                        &deps,
                    ));
                    let tv = tool_visual_state.clone();
                    tool_visual_snap_lines_points.replace_with(Property::computed(
                        move || tv.get().snap_lines.points,
                        &deps,
                    ));
                    let tv = tool_visual_state.clone();
                    tool_visual_event_blocker_active.replace_with(Property::computed(
                        move || tv.get().event_blocker_active,
                        &deps,
                    ));
                    tool_visual_state
                } else {
                    tool_visual_snap_lines_vertical.replace_with(Property::default());
                    tool_visual_snap_lines_horizontal.replace_with(Property::default());
                    tool_visual_snap_lines_points.replace_with(Property::default());
                    tool_visual_event_blocker_active.replace_with(Property::new(true));
                    // Default ToolVisualziation behavior
                    let deps = [mouse_pos.untyped(), world_transform.untyped()];
                    let mouse_pos = mouse_pos.clone();
                    let ctx = ctxp.clone();
                    Property::computed(
                        move || {
                            let (hit, to_glass) = model::with_action_context(&ctx, |ac| {
                                (
                                    ac.raycast_glass(mouse_pos.get(), RaycastMode::Top, &[]),
                                    ac.glass_transform(),
                                )
                            });
                            ToolVisualizationState {
                                rect_tool: Default::default(),
                                outline: hit
                                    .map(|h| {
                                        PathOutline::from_bounds(
                                            TransformAndBounds {
                                                transform: to_glass.get(),
                                                bounds: (1.0, 1.0),
                                            } * h.transform_and_bounds().get(),
                                        )
                                    })
                                    .unwrap_or_default(),
                                snap_lines: Default::default(),
                                event_blocker_active: true,
                            }
                        },
                        &deps,
                    )
                });
                true
            },
            &deps,
        ));

        let dt = borrow!(ctx.designtime);
        let manifest_ver = dt.get_manifest_version();
        let open_container =
            model::read_app_state_with_derived(|_, derived| derived.open_container.clone());
        let deps = [manifest_ver.untyped(), open_container.untyped()];
        // make scroller not clip if a child is selected
        // for now only scroller needs somewhat special behavior
        // might want to create more general double click framework at some point
        let ctx = ctx.clone();
        self.scroller_manifest_version_listener
            .replace_with(Property::computed(
                move || {
                    let open = open_container.get();

                    for node in open {
                        if let Some(node_interface) =
                            ctx.get_nodes_by_global_id(node.clone()).into_iter().next()
                        {
                            let open_container = open_container.clone();
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
                    false
                },
                &deps,
            ));
    }

    pub fn on_pre_render(&mut self, ctx: &NodeContext) {
        // update if dirty
        self.on_tool_change.get();

        // WARNING: this needs to be delayed a few frames,
        // if not the open container get's called before the userland root exists
        if ctx.frames_elapsed.get() > 2 {
            self.scroller_manifest_version_listener.get();
        };
    }

    pub fn context_menu(&mut self, _ctx: &NodeContext, args: Event<ContextMenu>) {
        args.prevent_default();
    }

    pub fn handle_double_click(&mut self, ctx: &NodeContext, event: Event<DoubleClick>) {
        // if a ControlPoint was double clicked, don't handle glass double click
        if event.cancelled() {
            return;
        }
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
            let builder = dt.get_orm_mut().get_node(
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

    pub fn handle_mouse_down(&mut self, ctx: &NodeContext, args: Event<MouseDown>) {
        let prevent_default = || args.prevent_default();
        model::perform_action(
            &crate::model::action::pointer::MouseEntryPointAction {
                prevent_default: &prevent_default,
                event: Pointer::Down,
                button: args.mouse.button.clone(),
                point: Point2::new(args.mouse.x, args.mouse.y),
            },
            ctx,
        );
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
                    node_layout: model::action::orm::NodeLayoutSettings::KeepScreenBounds {
                        node_transform_and_bounds: &TransformAndBounds {
                            transform: AxisAlignedBox::new(cw + v, cw - v).as_transform(),
                            bounds: (1.0, 1.0),
                        }
                        .as_pure_size(),
                        parent_transform_and_bounds: &parent.transform_and_bounds.get(),
                        node_decomposition_config: &Default::default(),
                    },
                    designer_node_type: DesignerNodeType::Image,
                    builder_extra_commands: Some(&|builder| {
                        builder.set_property(
                            "source",
                            &format!("ImageSource::Url(\"assets/{}\")", event.args.name),
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
    // rectangle drawing tool. Used during object creation
    // and for multi-select
    pub rect_tool: RectTool,
    // Highlight around an object when mouse is over
    pub outline: Vec<PathElement>,

    pub snap_lines: SnapInfo,
    // only dissabled when we need to interact with the nodes in the glass,
    // for example when editing text
    pub event_blocker_active: bool,
}

impl Default for ToolVisualizationState {
    fn default() -> Self {
        Self {
            rect_tool: Default::default(),
            outline: Default::default(),
            snap_lines: Default::default(),
            event_blocker_active: true,
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
    pub fill: Color,
    pub stroke: Color,
}

impl Default for RectTool {
    fn default() -> Self {
        RectTool {
            x: Size::ZERO(),
            y: Size::ZERO(),
            width: Size::ZERO(),
            height: Size::ZERO(),
            fill: Color::TRANSPARENT,
            stroke: Color::TRANSPARENT,
        }
    }
}
