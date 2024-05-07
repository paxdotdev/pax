use pax_engine::api::{ButtonClick, Click, Event, NodeContext, Numeric, Size, StringBox};
use pax_engine::math::Point2;
use pax_engine::*;
use pax_std::components::Stacker;
use pax_std::primitives::{Group, Image, Rectangle};
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::channel;

use pax_std::primitives::Button;
use pax_std::primitives::Ellipse;
use std::rc::Rc;

pub mod toolbar_item;
use crate::llm_interface::OpenLLMPrompt;
use crate::math::coordinate_spaces::Glass;
use crate::model::action::{Action, ActionContext};
use crate::model::{self, Component, ProjectMode, Tool, ToolBehaviour};
use crate::ProjectMsg;
use anyhow::Result;
use model::action::CanUndo;
use toolbar_item::ToolbarItemVisual;

#[pax]
#[file("controls/toolbar/mod.pax")]
pub struct Toolbar {
    pub selected_ind: Property<Numeric>,
    pub entries: Property<Vec<ToolbarItemView>>,
    pub dropdown_active: Property<bool>,
    pub dropdown_entries: Property<Vec<ToolbarItemView>>,
}

enum ToolbarEvent {
    SelectTool(Tool),
    PerformAction(Box<dyn Fn() -> Box<dyn Action>>),
}

pub struct SelectTool {
    pub tool: Tool,
}

impl Action for SelectTool {
    fn perform(self: Box<Self>, ctx: &mut model::action::ActionContext) -> Result<CanUndo> {
        ctx.app_state.selected_tool.set(self.tool);
        Ok(CanUndo::No)
    }
}

#[pax]
pub struct ToolbarItemView {
    pub background: bool,
    pub not_dummy: bool,
    pub icon: StringBox,
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
    icon: String,
    event: ToolbarEvent,
}

pub enum ToolbarClickEvent {
    Select(usize, usize),
    Dropdown(usize),
}

thread_local! {
    static TOOLBAR_CHANNEL: RefCell<Option<ToolbarClickEvent>> = RefCell::new(None);
    static TOOLBAR_ENTRIES: RefCell<Vec<ToolbarEntry>> = RefCell::new(
        vec![
            ToolbarEntry {
                items: vec![
                    ToolbarItem {
                        icon: "assets/icons/toolbar/icon-09-pointer.png".to_string(),
                        event: ToolbarEvent::SelectTool(Tool::Pointer)
                    }
                ]
            },
            ToolbarEntry {
                items: vec![
                    ToolbarItem {
                        icon: "assets/icons/toolbar/icon-10-brush.png".to_string(),
                        event: ToolbarEvent::SelectTool(Tool::TodoTool)
                    },
                    ToolbarItem {
                        icon: "assets/icons/toolbar/icon-11-pen.png".to_string(),
                        event: ToolbarEvent::SelectTool(Tool::TodoTool)
                    }
                ]
            },
            ToolbarEntry {
                items: vec![
                    ToolbarItem {
                        icon: "assets/icons/toolbar/icon-12-rect.png".to_string(),
                        event: ToolbarEvent::SelectTool(Tool::CreateComponent(Component::Rectangle))
                    },
                    ToolbarItem {
                        icon: "assets/icons/toolbar/icon-12-ellipse.png".to_string(),
                        event: ToolbarEvent::SelectTool(Tool::CreateComponent(Component::Ellipse))
                    }
                ]
            },
            ToolbarEntry {
                items: vec![
                    ToolbarItem {
                        icon: "assets/icons/toolbar/icon-14-text.png".to_string(),
                        event: ToolbarEvent::SelectTool(Tool::CreateComponent(Component::Text))
                    },
                ]
            },
            ToolbarEntry {
                items: vec![
                    ToolbarItem {
                        icon: "assets/icons/toolbar/icon-15-checkbox.png".to_string(),
                        event: ToolbarEvent::SelectTool(Tool::TodoTool)
                    },
                ]
            },
            ToolbarEntry {
                items: vec![
                    ToolbarItem {
                        icon: "assets/icons/toolbar/icon-16-speech.png".to_string(),
                        event: ToolbarEvent::PerformAction(Box::new(|| Box::new(OpenLLMPrompt { require_meta: false })))
                    },
                ]
            },
        ]
    );
}

impl Toolbar {
    pub fn on_mount(&mut self, _ctx: &NodeContext) {
        TOOLBAR_ENTRIES.with_borrow(|entries| {
            let mut entry_views = vec![];
            for (row, entry) in entries.iter().enumerate() {
                let first = entry.items.first().unwrap();
                entry_views.push(ToolbarItemView {
                    background: false,
                    not_dummy: true,
                    icon: StringBox::from(first.icon.clone()),
                    more_than_one_item: entry.items.len() > 1,
                    row,
                    col: 0,
                    x: Size::Pixels((row * 65).into()),
                    y: Size::Pixels(0.into()),
                });
            }
            self.entries.set(entry_views);
        });
    }

    pub fn update_view(&mut self, ctx: &NodeContext) {
        //HACK: pre dirty-dag
        let mut update = false;
        TOOLBAR_CHANNEL.with_borrow_mut(|store| {
            if let Some(event) = store.take() {
                self.dropdown_active.set(false);
                match event {
                    ToolbarClickEvent::Select(row, col) => {
                        let action = TOOLBAR_ENTRIES.with_borrow(|entries| {
                            let event = &entries[row].items[col].event;
                            match event {
                                &ToolbarEvent::SelectTool(tool) => Box::new(SelectTool { tool }),
                                ToolbarEvent::PerformAction(action_factory) => action_factory(),
                            }
                        });
                        model::perform_action(action, ctx);
                    }
                    ToolbarClickEvent::Dropdown(row) => {
                        self.dropdown_active.set(true);
                        TOOLBAR_ENTRIES.with_borrow(|entries| {
                            let items = &entries[row].items;
                            self.dropdown_entries.set(
                                items
                                    .iter()
                                    .enumerate()
                                    .map(|(col, item)| ToolbarItemView {
                                        background: true,
                                        not_dummy: true,
                                        icon: StringBox::from(item.icon.to_owned()),
                                        more_than_one_item: false,
                                        row,
                                        col,
                                        x: Size::Pixels((row * 65).into()),
                                        y: Size::Pixels((col * 65).into()),
                                    })
                                    .collect(),
                            );
                        });
                    }
                }
            };
        });
        model::read_app_state(|app_state| {
            //TODOdag
            let tool = app_state.selected_tool.get();
            TOOLBAR_ENTRIES.with_borrow(|entries| {
                'outer: for (row, entry) in entries.iter().enumerate() {
                    for (col, item) in entry.items.iter().enumerate() {
                        if let ToolbarEvent::SelectTool(toolbar_tool) = item.event {
                            if toolbar_tool == tool {
                                self.selected_ind.set(Numeric::from(row));
                                let mut entries = self.entries.get();
                                entries[row] = ToolbarItemView {
                                    background: false,
                                    not_dummy: true,
                                    icon: StringBox::from(item.icon.clone()),
                                    more_than_one_item: entry.items.len() > 1,
                                    row,
                                    col,
                                    x: Size::Pixels((row * 65).into()),
                                    y: Size::Pixels(0.into()),
                                };
                                self.entries.set(entries);
                                break 'outer;
                            }
                        }
                    }
                }
            });
        });
    }
}
