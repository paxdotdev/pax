use pax_engine::api::Fill;
use pax_engine::api::*;
use pax_engine::math::{Point2, Vector2};
use pax_engine::*;
use pax_manifest::{PaxType, TemplateNodeId, TypeId, UniqueTemplateNodeIdentifier};
use pax_std::primitives::{Group, Path, Rectangle};
use serde::Deserialize;

use crate::controls::file_and_component_picker::SetLibraryState;
use crate::model::action::orm::CreateComponent;
use crate::model::action::world::Translate;
use crate::model::tools::SelectNodes;
use crate::model::AppState;
use crate::{model, SetStage, StageInfo, ROOT_PROJECT_ID, USER_PROJ_ROOT_IMPORT_PATH};

use crate::math::coordinate_spaces::{self, World};
use crate::math::{self, AxisAlignedBox};
use crate::model::action::pointer::Pointer;
use crate::model::action::{Action, ActionContext, CanUndo, RaycastMode};
use crate::model::input::Dir;

pub mod control_point;
mod tool_editors;
pub mod wireframe_editor;
use control_point::ControlPoint;

use anyhow::anyhow;
use wireframe_editor::WireframeEditor;

use self::tool_editors::TextEdit;

#[pax]
#[custom(Default)]
#[file("glass/mod.pax")]
pub struct Glass {
    // rect tool state
    pub is_rect_tool_active: Property<bool>,
    pub rect_tool: Property<RectTool>,
}

pub struct SetEditingComponent(pub TypeId);

impl Action for SetEditingComponent {
    fn perform(self: Box<Self>, ctx: &mut ActionContext) -> anyhow::Result<CanUndo> {
        let type_id = self.0;

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
        ctx.execute(SetLibraryState { open: false })?;

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
        ctx.app_state.selected_component_id.set(type_id);
        ctx.app_state.tool_behaviour.set(None);
        Ok(CanUndo::No)
    }
}

impl Glass {
    pub fn update_tool_visual(&mut self, _ctx: &NodeContext) {
        model::read_app_state(|app_state| {
            // Draw current tool visuals
            // this could be factored out into it's own component as well eventually
            app_state.tool_behaviour.read(|tool| {
                if let Some(tool) = tool {
                    tool.borrow().visualize(self);
                } else {
                    self.is_rect_tool_active.set(false);
                }
            });
        });
    }

    pub fn context_menu(&mut self, _ctx: &NodeContext, args: Event<ContextMenu>) {
        args.prevent_default();
    }

    pub fn handle_double_click(&mut self, ctx: &NodeContext, args: Event<DoubleClick>) {
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
                Some("pax_designer::pax_reexports::pax_std::primitives::Text") => {
                    model::perform_action(TextEdit { uid }, ctx);
                }
                Some(
                    "pax_designer::pax_reexports::pax_std::primitives::Group"
                    | "pax_designer::pax_reexports::pax_std::stacker::Stacker",
                ) => {
                    model::with_action_context(ctx, |ax| {
                        let hit = ax.raycast_glass(
                            ax.glass_transform().get()
                                * Point2::<Window>::new(args.mouse.x, args.mouse.y),
                            RaycastMode::DrillOne,
                            &[],
                        );
                        if let Some(hit) = hit {
                            if let Err(e) = ax.execute(SelectNodes {
                                ids: &[hit.global_id().unwrap().get_template_node_id()],
                                overwrite: false,
                            }) {
                                log::warn!("failed to drill into group: {}", e);
                            };
                        }
                    });
                }
                // Assume it's a component if it didn't have a custom impl for double click behaviour
                Some(_) => model::perform_action(SetEditingComponent(node_id), ctx),
                None => (),
            }
        }
    }

    pub fn handle_mouse_down(&mut self, ctx: &NodeContext, args: Event<MouseDown>) {
        let prevent_default = || args.prevent_default();
        model::perform_action(
            crate::model::action::pointer::PointerAction {
                prevent_default: &prevent_default,
                event: Pointer::Down,
                button: args.mouse.button.clone(),
                point: Point2::new(args.mouse.x, args.mouse.y),
            },
            ctx,
        );
    }

    pub fn handle_mouse_move(&mut self, ctx: &NodeContext, args: Event<MouseMove>) {
        let prevent_default = || args.prevent_default();
        model::perform_action(
            crate::model::action::pointer::PointerAction {
                prevent_default: &prevent_default,
                event: Pointer::Move,
                button: args.mouse.button.clone(),
                point: Point2::new(args.mouse.x, args.mouse.y),
            },
            ctx,
        );
    }

    pub fn handle_mouse_up(&mut self, ctx: &NodeContext, args: Event<MouseUp>) {
        let prevent_default = || args.prevent_default();
        model::perform_action(
            crate::model::action::pointer::PointerAction {
                prevent_default: &prevent_default,
                event: Pointer::Up,
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
            if let Err(e) = ac.execute(Translate {
                translation: Vector2::new(args.delta_x, args.delta_y),
                original_transform: original,
            }) {
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
        model::with_action_context(ctx, |ac| {
            let cw = ac.world_transform()
                * ac.glass_transform().get()
                * Point2::new(event.args.x, event.args.y);
            let v = Vector2::new(150.0, 150.0);
            if let Err(e) = ac.execute(CreateComponent {
                bounds: AxisAlignedBox::new(cw + v, cw - v),
                type_id: TypeId::build_singleton(
                    "pax_designer::pax_reexports::pax_std::primitives::Image",
                    None,
                ),
                custom_props: vec![("path", &format!("\"assets/{}\"", event.args.name))],
            }) {
                log::warn!("failed to create image: {}", e);
            }
        });
    }
}

impl Default for Glass {
    fn default() -> Self {
        Self {
            is_rect_tool_active: Property::new(false),
            rect_tool: Default::default(),
        }
    }
}

#[pax]
// #[derive(Debug)]
pub struct RectTool {
    pub x: Size,
    pub y: Size,
    pub width: Size,
    pub height: Size,
    pub fill: Color,
    pub stroke: Color,
}
