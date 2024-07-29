use anyhow::anyhow;
use pax_engine::api::Fill;
use pax_engine::api::*;
use pax_engine::layout::TransformAndBounds;
use pax_engine::math::{Point2, Vector2};
use pax_engine::*;
use pax_manifest::{PaxType, TemplateNodeId, TypeId, UniqueTemplateNodeIdentifier};
use pax_std::*;
use serde::Deserialize;

use crate::controls::file_and_component_picker::SetLibraryState;
use crate::model::action::orm::CreateComponent;
use crate::model::action::world::Translate;
use crate::model::tools::SelectNodes;
use crate::model::{AppState, GlassNode};
use crate::{model, SetStage, StageInfo, ROOT_PROJECT_ID, USER_PROJ_ROOT_IMPORT_PATH};

use crate::math::coordinate_spaces::{self, World};
use crate::math::{self, AxisAlignedBox};
use crate::model::action::pointer::Pointer;
use crate::model::action::{Action, ActionContext, RaycastMode};
use crate::model::input::Dir;

pub mod control_point;
pub mod outline;
pub mod tool_editors;
pub mod wireframe_editor;

pub use self::tool_editors::TextEdit;
use control_point::ControlPoint;
use outline::PathOutline;
use wireframe_editor::WireframeEditor;

#[pax]
#[file("glass/mod.pax")]
pub struct Glass {
    pub tool_visual: Property<ToolVisualizationState>,
    pub on_tool_change: Property<bool>,
}

impl Glass {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        let tool_behaviour = model::read_app_state(|app_state| app_state.tool_behaviour.clone());
        let deps = [tool_behaviour.untyped()];
        let tool_visual = self.tool_visual.clone();
        let (mouse_pos, world_transform) = model::read_app_state(|app_state| {
            (
                app_state.mouse_position.clone(),
                app_state.glass_to_world_transform.clone(),
            )
        });
        let ctx = ctx.clone();
        self.on_tool_change.replace_with(Property::computed(
            move || {
                tool_visual.replace_with(if let Some(tool_behaviour) = tool_behaviour.get() {
                    tool_behaviour.borrow_mut().get_visual()
                } else {
                    // Default ToolVisualziation behaviour
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
            let builder = dt.get_orm_mut().get_node(uid.clone())?;
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
                                overwrite: false,
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
                            let node = ac.get_glass_node_by_global_id(&uid);
                            let open_containers = ac.derived_state.open_containers.clone();
                            let id = node.id.clone();
                            node.raw_node_interface
                                .with_properties(|scroller: &mut Scroller| {
                                    let deps = [open_containers.untyped()];
                                    scroller._clip_content.replace_with(Property::computed(
                                        move || {
                                            let is_open = open_containers.get().contains(&id);
                                            !is_open
                                        },
                                        &deps,
                                    ));
                                })
                                .unwrap();
                        }
                    });
                }
                // Assume it's a component if it didn't have a custom impl for double click behaviour
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

    pub fn handle_key_down(&mut self, ctx: &NodeContext, args: Event<KeyDown>) {
        model::process_keyboard_input(ctx, Dir::Down, args.keyboard.key.clone());
    }

    pub fn handle_key_up(&mut self, ctx: &NodeContext, args: Event<KeyUp>) {
        model::process_keyboard_input(ctx, Dir::Up, args.keyboard.key.clone());
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
        let parent = ctx
            .get_nodes_by_id(ROOT_PROJECT_ID)
            .into_iter()
            .next()
            .unwrap();
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

        let user_import_prefix = format!("{}::", USER_PROJ_ROOT_IMPORT_PATH);
        let is_userland_component = type_id
            .import_path()
            .is_some_and(|p| p.starts_with(&user_import_prefix));

        let is_mock = matches!(type_id.get_pax_type(), PaxType::BlankComponent { .. });

        if !is_userland_component && !is_mock {
            return Err(anyhow!(
                "tried to edit a non-userland comp: {:?}",
                type_id.import_path()
            ));
        }
        SetLibraryState { open: false }.perform(ctx)?;

        // TODO set stage defaults for opened component using "SetStage" action

        let mut dt = borrow_mut!(ctx.engine_context.designtime);
        let node = ctx
            .engine_context
            .get_nodes_by_id(ROOT_PROJECT_ID)
            .into_iter()
            .next()
            .ok_or(anyhow!("couldn't find node with userland id"))?;
        let mut builder = dt
            .get_orm_mut()
            .get_node(
                node.global_id()
                    .ok_or(anyhow!("expanded node doesn't have global id"))?,
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
        ctx.app_state.tool_behaviour.set(None);
        Ok(())
    }
}

#[pax]
pub struct ToolVisualizationState {
    pub rect_tool: RectTool,
    pub outline: Vec<PathElement>,
}

#[pax]
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
