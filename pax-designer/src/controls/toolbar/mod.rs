use pax_engine::api::{
    borrow_mut, ButtonClick, Click, Event, Interpolatable, NodeContext, Numeric, Size,
};
use pax_engine::math::Point2;
use pax_engine::*;
use pax_std::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::channel;

use std::rc::Rc;

pub mod toolbar_item;
use crate::llm_interface::SetLLMPromptState;
use crate::math::coordinate_spaces::Glass;
use crate::math::SizeUnit;
use crate::model::action::tool::SetToolBehaviour;
use crate::model::action::world::SelectNodes;
use crate::model::action::{Action, ActionContext};
use crate::model::{self, ProjectMode, Tool, ToolBehavior, ToolbarComponent};
use crate::ProjectMsg;
use anyhow::Result;
use toolbar_item::ToolbarItemVisual;

#[pax]
#[engine_import_path("pax_engine")]
#[file("controls/toolbar/mod.pax")]
pub struct Toolbar {
    pub selected_ind: Property<usize>,
    pub entries: Property<Vec<ToolbarItemView>>,
    pub dropdown_entries: Property<Vec<ToolbarItemView>>,
}

#[pax]
#[engine_import_path("pax_engine")]
pub struct ToolbarItemView {
    pub background: bool,
    pub tooltip: String,
    pub icon: String,
    pub more_than_one_item: bool,
    pub row: usize,
    pub col: usize,
    pub x: Size,
    pub y: Size,
}

struct ToolbarEntry {
    items: Vec<ToolbarItem>,
}

struct ToolbarItem {
    icon: &'static str,
    tooltip: &'static str,
    event: ToolbarEvent,
}

#[pax]
#[engine_import_path("pax_engine")]
#[derive(PartialEq)]
pub enum ToolbarClickEvent {
    Select(usize, usize),
    Dropdown(usize),
    #[default]
    CloseDropdown,
}

pub fn dropdown_is_in_open_state() -> bool {
    CLICK_PROP.with(|p| matches!(p.get(), ToolbarClickEvent::Dropdown(_)))
}

pub struct CloseDropdown;

impl Action for CloseDropdown {
    fn perform(&self, _ctx: &mut ActionContext) -> Result<()> {
        CLICK_PROP.with(|p| {
            if matches!(p.get(), ToolbarClickEvent::Dropdown(_)) {
                p.set(ToolbarClickEvent::CloseDropdown)
            }
        });
        Ok(())
    }
}

thread_local! {
    static CLICK_PROP: Property<ToolbarClickEvent> = Property::new(ToolbarClickEvent::CloseDropdown);
    static TOOLBAR_ENTRIES: Vec<ToolbarEntry> =
        vec![
            ToolbarEntry {
                items: vec![
                    ToolbarItem {
                        icon: "assets/icons/icon-pointer-percent.png",
                        tooltip: "Pointer Tool Percent",
                        event: ToolbarEvent::SelectTool(Tool::PointerPercent)
                    },
                    ToolbarItem {
                        icon: "assets/icons/icon-pointer-px.png",
                        tooltip: "Pointer Tool Pixels",
                        event: ToolbarEvent::SelectTool(Tool::PointerPixels)
                    }
                ]
            },
            ToolbarEntry {
                items: vec![
                    ToolbarItem {
                        icon: "assets/icons/icon-brush.png",
                        tooltip: "PaintBrush Tool",
                        event: ToolbarEvent::SelectTool(Tool::Paintbrush)
                    },
                    // ToolbarItem {
                    //     icon: "assets/icons/toolbar/icon-11-pen.png",
                    //     event: ToolbarEvent::SelectTool(Tool::TodoTool)
                    // }
                ]
            },
            ToolbarEntry {
                items: vec![
                    ToolbarItem {
                        icon: "assets/icons/icon-rectangle.png",
                        tooltip: "Rectangle Creation Tool",
                        event: ToolbarEvent::SelectTool(Tool::CreateComponent(ToolbarComponent::Rectangle))
                    },
                    ToolbarItem {
                        icon: "assets/icons/icon-ellipse.png",
                        tooltip: "Ellipse Creation Tool",
                        event: ToolbarEvent::SelectTool(Tool::CreateComponent(ToolbarComponent::Ellipse))
                    }
                ]
            },
            ToolbarEntry {
                items: vec![
                    ToolbarItem {
                        icon: "assets/icons/icon-text.png",
                        tooltip: "Text Creation Tool",
                        event: ToolbarEvent::SelectTool(Tool::CreateComponent(ToolbarComponent::Text))
                    },
                ]
            },
            ToolbarEntry {
                items: vec![
                    ToolbarItem {
                        icon: "assets/icons/icon-stacker.png",
                        tooltip: "Stacker Creation Tool",
                        event: ToolbarEvent::SelectTool(Tool::CreateComponent(ToolbarComponent::Stacker))
                    },
                    ToolbarItem {
                        icon: "assets/icons/icon-scroller.png",
                        tooltip: "Scroller Creation Tool",
                        event: ToolbarEvent::SelectTool(Tool::CreateComponent(ToolbarComponent::Scroller))
                    },
                ]
            },
            ToolbarEntry {
                items: vec![
                    ToolbarItem {
                        icon: "assets/icons/icon-checkbox.png",
                        tooltip: "Checkbox Creation Tool",
                        event: ToolbarEvent::SelectTool(Tool::CreateComponent(ToolbarComponent::Checkbox))
                    },
                    ToolbarItem {
                        icon: "assets/icons/icon-textbox.png",
                        tooltip: "Textbox Creation Tool",
                        event: ToolbarEvent::SelectTool(Tool::CreateComponent(ToolbarComponent::Textbox))
                    },
                    ToolbarItem {
                        icon: "assets/icons/icon-button.png",
                        tooltip: "Button Creation Tool",
                        event: ToolbarEvent::SelectTool(Tool::CreateComponent(ToolbarComponent::Button))
                    },
                    ToolbarItem {
                        icon: "assets/icons/icon-slider.png",
                        tooltip: "Slider Creation Tool",
                        event: ToolbarEvent::SelectTool(Tool::CreateComponent(ToolbarComponent::Slider))
                    },
                    ToolbarItem {
                        icon: "assets/icons/icon-dropdown.png",
                        tooltip: "Dropdown Creation Tool",
                        event: ToolbarEvent::SelectTool(Tool::CreateComponent(ToolbarComponent::Dropdown))
                    },
                    ToolbarItem {
                        icon: "assets/icons/icon-component.png",
                        tooltip: "RadioSet Creation Tool",
                        event: ToolbarEvent::SelectTool(Tool::CreateComponent(ToolbarComponent::RadioSet))
                    },
                ]
            },
            ToolbarEntry {
                items: vec![
                    ToolbarItem {
                        icon: "assets/icons/icon-robot.png",
                        tooltip: "AI Assistance",
                        event: ToolbarEvent::PerformAction(Box::new(SetLLMPromptState(true)))
                    },
                ]
            },
        ];
}

impl Toolbar {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        self.set_entries_from_def();
        self.bind_dropdown_entries(ctx.clone());
        self.bind_selected_ind();
    }

    fn set_entries_from_def(&mut self) {
        let entries = TOOLBAR_ENTRIES.with(|entries| {
            entries
                .iter()
                .enumerate()
                .map(|(row, entry)| {
                    let first = entry.items.first().unwrap();
                    ToolbarItemView {
                        background: false,
                        tooltip: String::from(first.tooltip),
                        icon: String::from(first.icon),
                        more_than_one_item: entry.items.len() > 1,
                        row,
                        col: 0,
                        x: Size::Pixels((row * 55).into()),
                        y: Size::Pixels(0.into()),
                    }
                })
                .collect()
        });
        self.entries.replace_with(Property::new(entries));
    }

    fn bind_dropdown_entries(&mut self, ctx: NodeContext) {
        let toolbar_click = CLICK_PROP.with(|p| p.clone());
        let deps = [toolbar_click.untyped()];
        self.dropdown_entries.replace_with(Property::computed(
            move || match toolbar_click.get() {
                ToolbarClickEvent::Select(row, col) => {
                    TOOLBAR_ENTRIES.with(|entries| {
                        let event = &entries[row].items[col].event;
                        match event {
                            &ToolbarEvent::SelectTool(tool) => {
                                model::perform_action(&SelectTool { tool }, &ctx)
                            }
                            ToolbarEvent::PerformAction(action) => {
                                model::perform_action(action.as_ref(), &ctx)
                            }
                        }
                    });
                    vec![]
                }
                ToolbarClickEvent::CloseDropdown => vec![],
                ToolbarClickEvent::Dropdown(row) => TOOLBAR_ENTRIES.with(|entries| {
                    entries[row]
                        .items
                        .iter()
                        .enumerate()
                        .map(|(col, item)| ToolbarItemView {
                            background: true,
                            tooltip: String::from(item.tooltip),
                            icon: String::from(item.icon),
                            more_than_one_item: false,
                            row,
                            col,
                            x: Size::Pixels((row * 55).into()),
                            y: Size::Pixels(((col + 1) * 55).into()),
                        })
                        .collect()
                }),
            },
            &deps,
        ));
    }

    fn bind_selected_ind(&mut self) {
        model::read_app_state(|app_state| {
            let tool = app_state.selected_tool.clone();
            let entries = self.entries.clone();
            let deps = [tool.untyped()];
            self.selected_ind.replace_with(Property::computed(
                move || {
                    TOOLBAR_ENTRIES.with(|entries_template| {
                        for (row, entry) in entries_template.iter().enumerate() {
                            for (col, item) in entry.items.iter().enumerate() {
                                if let ToolbarEvent::SelectTool(toolbar_tool) = item.event {
                                    if toolbar_tool == tool.get() {
                                        entries.update(|entries| {
                                            entries[row] = ToolbarItemView {
                                                background: false,
                                                tooltip: String::from(item.tooltip),
                                                icon: String::from(item.icon),
                                                more_than_one_item: entry.items.len() > 1,
                                                row,
                                                col,
                                                x: Size::Pixels((row * 55).into()),
                                                y: Size::Pixels(0.into()),
                                            };
                                        });
                                        return row;
                                    }
                                }
                            }
                        }
                        0
                    })
                },
                &deps,
            ));
        });
    }

    pub fn on_unmount(&mut self, _ctx: &NodeContext) {
        CLICK_PROP.with(|p| p.set(ToolbarClickEvent::CloseDropdown));
    }
}
enum ToolbarEvent {
    SelectTool(Tool),
    #[allow(unused)]
    PerformAction(Box<dyn Action>),
}

pub struct SelectTool {
    pub tool: Tool,
}

impl Action for SelectTool {
    fn perform(&self, ctx: &mut model::action::ActionContext) -> Result<()> {
        // set px/percent mode if a new pointer tool is selected,
        // is there some better way of persisting this? (tool stack?)
        match self.tool {
            Tool::PointerPercent => ctx.app_state.unit_mode.set(SizeUnit::Percent),
            Tool::PointerPixels => ctx.app_state.unit_mode.set(SizeUnit::Pixels),
            _ => (),
        }
        ctx.app_state.selected_tool.set(self.tool);
        Ok(())
    }
}

pub struct FinishCurrentTool;

impl Action for FinishCurrentTool {
    fn perform(&self, ctx: &mut ActionContext) -> Result<()> {
        let current_tool = ctx.app_state.tool_behavior.get();
        let selected_tool = ctx.app_state.selected_tool.get();
        if current_tool.is_some() {
            SetToolBehaviour(None).perform(ctx)
        } else if !matches!(selected_tool, Tool::PointerPercent | Tool::PointerPixels) {
            ctx.app_state
                .selected_tool
                .set(match ctx.app_state.unit_mode.get() {
                    SizeUnit::Pixels => Tool::PointerPixels,
                    SizeUnit::Percent => Tool::PointerPercent,
                });
            Ok(())
        } else {
            SelectNodes {
                ids: &[],
                mode: model::action::world::SelectMode::DiscardOthers,
            }
            .perform(ctx)
        }
    }
}
