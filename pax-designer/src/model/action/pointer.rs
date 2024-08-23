use std::cell::RefCell;
use std::rc::Rc;

use super::{Action, ActionContext, RaycastMode};
use crate::context_menu::ContextMenuMsg;
use crate::math::coordinate_spaces::Glass;
use crate::math::SizeUnit;
use crate::model::action::world::Pan;
use crate::model::input::{InputEvent, ModifierKey};
use crate::model::tools::{CreateComponentTool, MovingTool, MultiSelectTool, ZoomToFitTool};
use crate::model::Component;
use crate::model::{action, Tool};
use crate::model::{AppState, StageInfo};
use crate::SetStage;
use anyhow::{anyhow, Result};
use pax_designtime::DesigntimeManager;
use pax_engine::api::{borrow, Color, MouseButton, Window};
use pax_engine::log;
use pax_engine::math::Point2;
use pax_engine::pax_manifest::{TypeId, Unit};

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
                MouseButton::Left => match ctx.app_state.selected_tool.get() {
                    Tool::PointerPercent | Tool::PointerPixels => {
                        (self.prevent_default)();
                        if let Some(hit) = ctx.raycast_glass(point_glass, RaycastMode::Top, &[]) {
                            tool_behavior.set(Some(Rc::new(RefCell::new(MovingTool::new(
                                ctx,
                                point_glass,
                                hit,
                            )))));
                        } else {
                            tool_behavior.set(Some(Rc::new(RefCell::new(MultiSelectTool::new(
                                ctx,
                                point_glass,
                            )))));
                        }
                    }
                    Tool::CreateComponent(component) => {
                        tool_behavior.set(Some(Rc::new(RefCell::new(match component {
                            Component::Rectangle => CreateComponentTool::new(
                                point_glass,
                                &TypeId::build_singleton(
                                    "pax_std::drawing::rectangle::Rectangle",
                                    None,
                                ),
                                0,
                                &[],
                                ctx,
                            ),
                            Component::Ellipse => CreateComponentTool::new(
                                point_glass,
                                &TypeId::build_singleton(
                                    "pax_std::drawing::ellipse::Ellipse",
                                    None,
                                ),
                                0,
                                &[],
                                ctx,
                            ),
                            Component::Text => CreateComponentTool::new(
                                point_glass,
                                &TypeId::build_singleton("pax_std::core::text::Text", None),
                                0,
                                &[],
                                ctx,
                            ),
                            Component::Scroller => CreateComponentTool::new(
                                point_glass,
                                &TypeId::build_singleton("pax_std::core::scroller::Scroller", None),
                                1,
                                &[("scroll_width", "100%"), ("scroll_height", "200%")],
                                ctx,
                            ),
                            Component::Stacker => CreateComponentTool::new(
                                point_glass,
                                &TypeId::build_singleton("pax_std::layout::stacker::Stacker", None),
                                5,
                                &[],
                                ctx,
                            ),

                            Component::Checkbox => CreateComponentTool::new(
                                point_glass,
                                &TypeId::build_singleton(
                                    "pax_std::forms::checkbox::Checkbox",
                                    None,
                                ),
                                0,
                                &[],
                                ctx,
                            ),
                            Component::Textbox => CreateComponentTool::new(
                                point_glass,
                                &TypeId::build_singleton("pax_std::forms::textbox::Textbox", None),
                                0,
                                &[],
                                ctx,
                            ),
                            Component::Button => CreateComponentTool::new(
                                point_glass,
                                &TypeId::build_singleton("pax_std::forms::button::Button", None),
                                0,
                                &[],
                                ctx,
                            ),
                            Component::Slider => CreateComponentTool::new(
                                point_glass,
                                &TypeId::build_singleton("pax_std::forms::slider::Slider", None),
                                0,
                                &[],
                                ctx,
                            ),
                            Component::Dropdown => CreateComponentTool::new(
                                point_glass,
                                &TypeId::build_singleton(
                                    "pax_std::forms::dropdown::Dropdown",
                                    None,
                                ),
                                0,
                                &[],
                                ctx,
                            ),
                            Component::RadioSet => CreateComponentTool::new(
                                point_glass,
                                &TypeId::build_singleton(
                                    "pax_std::forms::radio_set::RadioSet",
                                    None,
                                ),
                                0,
                                &[],
                                ctx,
                            ),
                        }))));
                    }
                    Tool::TodoTool => {
                        log::warn!("tool has no implemented behavior");
                    }
                },
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
                        // TODO this could most likely be done in a nicer way:
                        // make a tool "stack", and return to last tool here instead
                        ctx.app_state
                            .selected_tool
                            .set(match ctx.app_state.unit_mode.get() {
                                SizeUnit::Pixels => Tool::PointerPixels,
                                SizeUnit::Percent => Tool::PointerPercent,
                            });
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
