use std::cell::RefCell;
use std::rc::Rc;

use super::meta::Schedule;
use super::orm::{CreateComponent, NodeLayoutSettings};
use super::{Action, ActionContext, RaycastMode};
use crate::context_menu::ContextMenuMsg;
use crate::designer_node_type::DesignerNodeType;
use crate::glass::TextEdit;
use crate::math::coordinate_spaces::Glass;
use crate::math::SizeUnit;
use crate::model::action::world::Pan;
use crate::model::input::{InputEvent, ModifierKey};
use crate::model::tools::{
    CreateComponentTool, MovingTool, MultiSelectTool, PaintbrushTool, ZoomToFitTool,
};
use crate::model::ToolbarComponent;
use crate::model::{action, Tool};
use crate::model::{AppState, StageInfo};
use crate::SetStage;
use anyhow::{anyhow, Result};
use pax_designtime::DesigntimeManager;
use pax_engine::api::{borrow, borrow_mut, Color, MouseButton, Window};
use pax_engine::log;
use pax_engine::math::Point2;
use pax_engine::pax_manifest::{TreeIndexPosition, TypeId, Unit};

pub struct MouseEntryPointAction<'a> {
    pub prevent_default: &'a dyn Fn(),
    pub event: Pointer,
    pub button: MouseButton,
    pub point: Point2<Window>,
}

#[derive(Clone, PartialEq, Copy)]
pub enum Pointer {
    Down,
    Move,
    Up,
}

impl Action for MouseEntryPointAction<'_> {
    fn perform(&self, ctx: &mut ActionContext) -> Result<()> {
        let point_glass = ctx.glass_transform().get() * self.point;
        ctx.app_state.mouse_position.set(point_glass);
        let spacebar = ctx.app_state.modifiers.get().contains(&ModifierKey::Space);
        let zoom = ctx.app_state.modifiers.get().contains(&ModifierKey::Z);
        let tool_behavior = ctx.app_state.tool_behavior.clone();
        // Open context menu on right mouse button click no matter what
        if matches!(
            (self.event, self.button.clone()),
            (Pointer::Down, MouseButton::Right)
        ) {
            ContextMenuMsg::Open { pos: point_glass }.perform(ctx)?;
            return Ok(());
        }

        if matches!(self.event, Pointer::Down) {
            ContextMenuMsg::Close.perform(ctx)?;
        }

        // If no tool is active, activate a tool on mouse down
        if matches!(self.event, Pointer::Down) && tool_behavior.get().is_none() {
            match &self.button {
                MouseButton::Left if zoom => {
                    tool_behavior.set(Some(Rc::new(RefCell::new(ZoomToFitTool::new(point_glass)))));
                }
                MouseButton::Left if spacebar => {
                    tool_behavior.set(Some(Rc::new(RefCell::new(Pan {
                        start_point: point_glass,
                        original_transform: ctx.app_state.glass_to_world_transform.get(),
                    }))));
                }
                MouseButton::Middle => {
                    tool_behavior.set(Some(Rc::new(RefCell::new(Pan {
                        start_point: point_glass,
                        original_transform: ctx.app_state.glass_to_world_transform.get(),
                    }))));
                }
                MouseButton::Left => {
                    match ctx.app_state.selected_tool.get() {
                        Tool::PointerPercent | Tool::PointerPixels => {
                            (self.prevent_default)();
                            if let Some(hit) = ctx.raycast_glass(point_glass, RaycastMode::Top, &[])
                            {
                                tool_behavior.set(Some(Rc::new(RefCell::new(MovingTool::new(
                                    ctx,
                                    point_glass,
                                    hit,
                                )))));
                            } else {
                                tool_behavior.set(Some(Rc::new(RefCell::new(
                                    MultiSelectTool::new(ctx, point_glass),
                                ))));
                            }
                        }
                        Tool::CreateComponent(component) => {
                            tool_behavior.set(Some(Rc::new(RefCell::new(match component {
                            ToolbarComponent::Rectangle => CreateComponentTool::new(
                                point_glass,
                                DesignerNodeType::Rectangle,
                                ctx,
                            ),
                            ToolbarComponent::Ellipse => CreateComponentTool::new(
                                point_glass,
                                DesignerNodeType::Ellipse,
                                ctx,
                            ),
                            ToolbarComponent::Text => CreateComponentTool::new(
                                point_glass,
                                DesignerNodeType::Text,
                                ctx,
                            )
                            .with_post_creation_hook(|ctx, post_creation_data| {
                                // Node doesn't exit yet in engine (and needs to to be able to
                                // set contenteditable to true) -> schedule for next frame.
                                Schedule {
                                    action: Rc::new(TextEdit {
                                        uid: post_creation_data.uid.clone(),
                                    }),
                                }
                                .perform(ctx)?;
                                Ok(())
                            }),
                            ToolbarComponent::Scroller => CreateComponentTool::new(
                                point_glass,
                                DesignerNodeType::Scroller,
                                ctx,
                            )
                            .with_extra_builder_commands(|builder| {
                                builder.set_property("scroll_height", "200%")
                            })
                            .with_post_creation_hook(|ctx, post_creation_data| {
                                CreateComponent {
                                    parent_id: &post_creation_data.uid,
                                    parent_index: TreeIndexPosition::Top,
                                    designer_node_type: DesignerNodeType::Rectangle,
                                    builder_extra_commands: Some(&|builder| {
                                        builder.set_property("fill", "GRAY")
                                    }),
                                    node_layout: Some(NodeLayoutSettings::Fill),
                                }
                                .perform(ctx)?;
                                Ok(())
                            }),
                            ToolbarComponent::Stacker => CreateComponentTool::new(
                                point_glass,
                                DesignerNodeType::Stacker,
                                ctx,
                            )
                            .with_post_creation_hook(|ctx, post_creation_data| {
                                for i in 1..=3 {
                                    let c = 210 - 60 * (i % 2);
                                    CreateComponent {
                                        parent_id: &post_creation_data.uid,
                                        parent_index: TreeIndexPosition::Top,
                                        designer_node_type: DesignerNodeType::Rectangle,
                                        builder_extra_commands: Some(&|builder| {
                                            builder.set_property(
                                                "fill",
                                                &format!("rgb({}, {}, {})", c, c, c),
                                            )
                                        }),
                                        node_layout: Some(NodeLayoutSettings::Fill),
                                    }
                                    .perform(ctx)?;
                                }
                                {
                                    let mut dt = borrow_mut!(ctx.engine_context.designtime);
                                    let mut node = dt
                                        .get_orm_mut()
                                        .get_node_builder(post_creation_data.uid.clone(), false)
                                        .ok_or_else(|| anyhow!("couldn't get stacker node"))?;
                                    if post_creation_data.bounds.width()
                                        > post_creation_data.bounds.height()
                                    {
                                        node.set_property(
                                            "direction",
                                            "StackerDireciton::Horizontal",
                                        )?;
                                        node.save().map_err(|e| anyhow!("failed to save while setting direction on stacker: {e}"))?;
                                    }
                                }
                                Ok(())
                            }),
                            ToolbarComponent::Checkbox => CreateComponentTool::new(
                                point_glass,
                                DesignerNodeType::Checkbox,
                                ctx,
                            ),
                            ToolbarComponent::Textbox => CreateComponentTool::new(
                                point_glass,
                                DesignerNodeType::Textbox,
                                ctx,
                            ),
                            ToolbarComponent::Button => CreateComponentTool::new(
                                point_glass,
                                DesignerNodeType::Button,
                                ctx,
                            ),
                            ToolbarComponent::Slider => CreateComponentTool::new(
                                point_glass,
                                DesignerNodeType::Slider,
                                ctx,
                            ),
                            ToolbarComponent::Dropdown => CreateComponentTool::new(
                                point_glass,
                                DesignerNodeType::Dropdown,
                                ctx,
                            ),
                            ToolbarComponent::RadioSet => CreateComponentTool::new(
                                point_glass,
                                DesignerNodeType::RadioSet,
                                ctx,
                            ),
                        }))));
                        }
                        Tool::Paintbrush => match PaintbrushTool::new(ctx) {
                            Ok(paint_tool) => {
                                tool_behavior.set(Some(Rc::new(RefCell::new(paint_tool))))
                            }
                            Err(e) => log::warn!("couldn't create path for paintbrush: {e}"),
                        },
                        Tool::TodoTool => {
                            log::warn!("tool has no implemented behavior");
                        }
                    }
                }
                _ => (),
            };
        }

        // Whatever tool behavior exists, let it do its thing
        let point = ctx.app_state.mouse_position.get();
        let reset = tool_behavior.read(|tool_behavior| {
            if let Some(tool) = tool_behavior {
                let mut tool = tool.borrow_mut();
                let res = match self.event {
                    Pointer::Down => tool.pointer_down(point, ctx),
                    Pointer::Move => tool.pointer_move(point, ctx),
                    Pointer::Up => tool.pointer_up(point, ctx),
                };
                match res {
                    std::ops::ControlFlow::Continue(_) => false,
                    std::ops::ControlFlow::Break(_) => {
                        if let Err(e) = tool.finish(ctx) {
                            log::warn!("finishing tool failed: {e}");
                        };
                        // TODO this could most likely be done in a nicer way:
                        // make a tool "stack", and return to last tool here
                        // instead. How to handle ToolBehavior vs Toolbar state
                        // if doing this? (needs to be synced). Associate each
                        // ToolBehavior with a Tool in toolbar?
                        // TODO might want to make a toolbar click immediately
                        // activate a tool, and then make the tool itself handle
                        // mousedown. (would allow for easier way for tool
                        // settings view to show on tool selection)
                        match ctx.app_state.selected_tool.get() {
                            Tool::Paintbrush => (),
                            _ => {
                                ctx.app_state.selected_tool.set(
                                    match ctx.app_state.unit_mode.get() {
                                        SizeUnit::Pixels => Tool::PointerPixels,
                                        SizeUnit::Percent => Tool::PointerPercent,
                                    },
                                );
                            }
                        }

                        true
                    }
                }
            } else {
                false
            }
        });
        if reset {
            tool_behavior.set(None);
        }
        Ok(())
    }
}
