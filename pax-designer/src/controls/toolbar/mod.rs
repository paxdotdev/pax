use pax_engine::api::{ButtonClick, Click, Event, Interpolatable, NodeContext, Numeric, Size};
use pax_engine::math::Point2;
use pax_engine::*;
use pax_std::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::channel;

use std::rc::Rc;

pub mod toolbar_item;
use crate::llm_interface::OpenLLMPrompt;
use crate::math::coordinate_spaces::Glass;
use crate::model::action::{Action, ActionContext};
use crate::model::{self, Component, ProjectMode, Tool, ToolBehavior};
use crate::ProjectMsg;
use anyhow::Result;
use toolbar_item::ToolbarItemVisual;

#[pax]
#[file("controls/toolbar/mod.pax")]
pub struct Toolbar {
    pub selected_ind: Property<usize>,
    pub entries: Property<Vec<ToolbarItemView>>,
    pub dropdown_entries: Property<Vec<ToolbarItemView>>,
}

#[pax]
pub struct ToolbarItemView {
    pub background: bool,
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
    event: ToolbarEvent,
}

#[pax]
#[derive(PartialEq)]
pub enum ToolbarClickEvent {
    Select(usize, usize),
    Dropdown(usize),
    #[default]
    None,
}

thread_local! {
    static CLICK_PROP: Property<ToolbarClickEvent> = Property::new(ToolbarClickEvent::None);
    static TOOLBAR_ENTRIES: Vec<ToolbarEntry> =
        vec![
            ToolbarEntry {
                items: vec![
                    ToolbarItem {
                        icon: "assets/icons/toolbar/icon-09-pointer-percent.png",
                        event: ToolbarEvent::SelectTool(Tool::PointerPercent)
                    },
                    ToolbarItem {
                        icon: "assets/icons/toolbar/icon-09-pointer.png",
                        event: ToolbarEvent::SelectTool(Tool::PointerPixels)
                    }
                ]
            },
            ToolbarEntry {
                items: vec![
                    ToolbarItem {
                        icon: "assets/icons/toolbar/icon-10-brush.png",
                        event: ToolbarEvent::SelectTool(Tool::TodoTool)
                    },
                    ToolbarItem {
                        icon: "assets/icons/toolbar/icon-11-pen.png",
                        event: ToolbarEvent::SelectTool(Tool::TodoTool)
                    }
                ]
            },
            ToolbarEntry {
                items: vec![
                    ToolbarItem {
                        icon: "assets/icons/toolbar/icon-12-rect.png",
                        event: ToolbarEvent::SelectTool(Tool::CreateComponent(Component::Rectangle))
                    },
                    ToolbarItem {
                        icon: "assets/icons/toolbar/icon-12-ellipse.png",
                        event: ToolbarEvent::SelectTool(Tool::CreateComponent(Component::Ellipse))
                    }
                ]
            },
            ToolbarEntry {
                items: vec![
                    ToolbarItem {
                        icon: "assets/icons/toolbar/icon-14-text.png",
                        event: ToolbarEvent::SelectTool(Tool::CreateComponent(Component::Text))
                    },
                ]
            },
            ToolbarEntry {
                items: vec![
                    ToolbarItem {
                        icon: "assets/icons/toolbar/icon-13-stacker.png",
                        event: ToolbarEvent::SelectTool(Tool::CreateComponent(Component::Stacker))
                    },
                    ToolbarItem {
                        icon: "assets/icons/tree/tree-icon-11-scroller.png",
                        event: ToolbarEvent::SelectTool(Tool::CreateComponent(Component::Scroller))
                    },
                ]
            },
            ToolbarEntry {
                items: vec![
                    ToolbarItem {
                        icon: "assets/icons/toolbar/icon-15-checkbox.png",
                        event: ToolbarEvent::SelectTool(Tool::CreateComponent(Component::Checkbox))
                    },
                    ToolbarItem {
                        icon: "assets/icons/tree/tree-icon-09-textbox.png",
                        event: ToolbarEvent::SelectTool(Tool::CreateComponent(Component::Textbox))
                    },
                    ToolbarItem {
                        icon: "assets/icons/tree/tree-icon-12-button.png",
                        event: ToolbarEvent::SelectTool(Tool::CreateComponent(Component::Button))
                    },
                    ToolbarItem {
                        icon: "assets/icons/tree/tree-icon-14-slider.png",
                        event: ToolbarEvent::SelectTool(Tool::CreateComponent(Component::Slider))
                    },
                    ToolbarItem {
                        icon: "assets/icons/tree/tree-icon-15-dropdown.png",
                        event: ToolbarEvent::SelectTool(Tool::CreateComponent(Component::Dropdown))
                    },
                    ToolbarItem {
                        icon: "assets/icons/tree/tree-icon-08-component.png",
                        event: ToolbarEvent::SelectTool(Tool::CreateComponent(Component::RadioSet))
                    },
                ]
            },
            ToolbarEntry {
                items: vec![
                    ToolbarItem {
                        icon: "assets/icons/toolbar/icon-16-speech.png",
                        event: ToolbarEvent::PerformAction(Box::new(|| Box::new(OpenLLMPrompt { require_meta: false })))
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
                        icon: String::from(first.icon),
                        more_than_one_item: entry.items.len() > 1,
                        row,
                        col: 0,
                        x: Size::Pixels((row * 65).into()),
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
                    let action = TOOLBAR_ENTRIES.with(|entries| {
                        let event = &entries[row].items[col].event;
                        match event {
                            &ToolbarEvent::SelectTool(tool) => Box::new(SelectTool { tool }),
                            ToolbarEvent::PerformAction(action_factory) => action_factory(),
                        }
                    });
                    model::perform_action(action.as_ref(), &ctx);
                    vec![]
                }
                ToolbarClickEvent::None => vec![],
                ToolbarClickEvent::Dropdown(row) => TOOLBAR_ENTRIES.with(|entries| {
                    entries[row]
                        .items
                        .iter()
                        .enumerate()
                        .map(|(col, item)| ToolbarItemView {
                            background: true,
                            icon: String::from(item.icon),
                            more_than_one_item: false,
                            row,
                            col,
                            x: Size::Pixels((row * 65).into()),
                            y: Size::Pixels((col * 65).into()),
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
                                                icon: String::from(item.icon),
                                                more_than_one_item: entry.items.len() > 1,
                                                row,
                                                col,
                                                x: Size::Pixels((row * 65).into()),
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
        CLICK_PROP.with(|p| p.set(ToolbarClickEvent::None));
    }
}
enum ToolbarEvent {
    SelectTool(Tool),
    PerformAction(Box<dyn Fn() -> Box<dyn Action>>),
}

pub struct SelectTool {
    pub tool: Tool,
}

impl Action for SelectTool {
    fn perform(&self, ctx: &mut model::action::ActionContext) -> Result<()> {
        ctx.app_state.selected_tool.set(self.tool);
        Ok(())
    }
}
