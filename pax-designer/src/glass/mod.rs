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
use crate::model::action::orm::CreateComponent;
use crate::model::action::world::Translate;
use crate::model::tools::SelectNodes;
use crate::model::{AppState, GlassNode};
use crate::{model, SetStage, StageInfo};

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
        let ctx = ctx.clone();
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
                    let ctx = ctx.clone();
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
    }

    pub fn on_pre_render(&mut self, _ctx: &NodeContext) {
        // update if dirty
        self.on_tool_change.get();
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
            let import_path = node_id.import_path();
            match import_path.as_ref().map(|v| v.as_str()) {
                Some("pax_std::core::text::Text") => {
                    model::perform_action(&TextEdit { uid }, ctx);
                }
                Some(
                    path @ ("pax_std::core::group::Group"
                    | "pax_std::layout::stacker::Stacker"
                    | "pax_std::core::scroller::Scroller"),
                ) => {
                    model::with_action_context(ctx, |ac| {
                        let hit = ac.raycast_glass(
                            ac.glass_transform().get()
                                * Point2::<Window>::new(event.mouse.x, event.mouse.y),
                            RaycastMode::DrillOne,
                            &[],
                        );
                        if let Some(hit) = hit {
                            if let Err(e) = (SelectNodes {
                                ids: &[hit.global_id().unwrap().get_template_node_id()],
                                mode: model::tools::SelectMode::Dynamic,
                            }
                            .perform(ac))
                            {
                                log::warn!("failed to drill into container: {}", e);
                            };
                        }
                        // make scroller not clip if a child is selected
                        // for now only scroller needs somewhat special behavior
                        // might want to create more general double click framework at some point
                        if path.contains("Scroller") {
                            let node = ac.get_glass_node_by_global_id(&uid).unwrap();
                            let open_containers = ac.derived_state.open_container.clone();
                            let id = node.id.clone();
                            node.raw_node_interface
                                .with_properties(|scroller: &mut Scroller| {
                                    let deps = [open_containers.untyped()];
                                    scroller._clip_content.replace_with(Property::computed(
                                        move || {
                                            let is_open = open_containers.get() == id;
                                            !is_open
                                        },
                                        &deps,
                                    ));
                                })
                                .unwrap();
                        }
                    });
                }
                // Assume it's a component if it didn't have a custom impl for double click behavior
                Some(_) => model::perform_action(&SetEditingComponent(node_id), ctx),
                None => (),
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

    pub fn handle_key_down(&mut self, ctx: &NodeContext, event: Event<KeyDown>) {
        event.prevent_default();
        model::process_keyboard_input(ctx, Dir::Down, event.keyboard.key.clone());
    }

    pub fn handle_key_up(&mut self, ctx: &NodeContext, event: Event<KeyUp>) {
        event.prevent_default();
        model::process_keyboard_input(ctx, Dir::Up, event.keyboard.key.clone());
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
        let parent = ctx.get_userland_root_expanded_node();
        model::with_action_context(ctx, |ac| {
            let parent = GlassNode::new(&parent, &ac.glass_transform());
            let cw = ac.glass_transform().get() * Point2::new(event.args.x, event.args.y);
            let v = Vector2::new(150.0, 150.0);
            if let Err(e) = (CreateComponent {
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
                type_id: &TypeId::build_singleton("pax_std::core::image::Image", None),
                custom_props: &[("path", &format!("\"assets/{}\"", event.args.name))],
                mock_children: 0,
            }
            .perform(ac))
            {
                log::warn!("failed to create image: {}", e);
            }
        });
    }
}

pub struct SetEditingComponent(pub TypeId);

impl Action for SetEditingComponent {
    fn perform(&self, ctx: &mut ActionContext) -> anyhow::Result<()> {
        let type_id = &self.0;

        let is_pax_std = type_id
            .import_path()
            .is_some_and(|p| p.starts_with("pax_std"));
   
        if is_pax_std {
            return Err(anyhow!(
                "tried to edit a non-userland comp: {:?}",
                type_id.import_path()
            ));
        }
        
        SetLibraryState { open: false }.perform(ctx)?;

        // TODO set stage defaults for opened component using "SetStage" action

        let mut dt = borrow_mut!(ctx.engine_context.designtime);
        let node = ctx.engine_context.get_userland_root_expanded_node();
        let mut builder = dt
            .get_orm_mut()
            .get_node(
                node.global_id()
                    .ok_or(anyhow!("expanded node doesn't have global id"))?,
                false,
            )
            .ok_or(anyhow!("no such node in manifest"))?;
        builder.set_type_id(&type_id);
        builder
            .save()
            .map_err(|_| anyhow!("builder couldn't save"))?;
        dt.reload_edit();
        // reset other relevant app state
        ctx.app_state
            .selected_template_node_ids
            .update(|v| v.clear());
        ctx.app_state.selected_component_id.set(type_id.clone());
        ctx.app_state.tool_behavior.set(None);
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
