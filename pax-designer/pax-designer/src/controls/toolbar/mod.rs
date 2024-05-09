use pax_engine::api::{ButtonClick, Click, Event, Interpolatable, NodeContext, Numeric, Size};
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
    pub selected_ind: Property<usize>,
    pub entries: Property<Vec<ToolbarItemView>>,
    pub dropdown_entries: Property<Vec<ToolbarItemView>>,

    pub click_ev: Property<ToolbarClickEvent>,
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
                        icon: "assets/icons/toolbar/icon-09-pointer.png",
                        event: ToolbarEvent::SelectTool(Tool::Pointer)
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
                        icon: "assets/icons/toolbar/icon-15-checkbox.png",
                        event: ToolbarEvent::SelectTool(Tool::TodoTool)
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
        TOOLBAR_ENTRIES.with(|entries| {
            let mut entry_views = vec![];
            for (row, entry) in entries.iter().enumerate() {
                let first = entry.items.first().unwrap();
                entry_views.push(ToolbarItemView {
                    background: false,
                    icon: String::from(first.icon),
                    more_than_one_item: entry.items.len() > 1,
                    row,
                    col: 0,
                    x: Size::Pixels((row * 65).into()),
                    y: Size::Pixels(0.into()),
                });
            }
            self.entries.set(entry_views);
        });

        let ctx = ctx.clone();
        let toolbar_click = self.click_ev.clone();
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
                    model::perform_action(action, &ctx);
                    vec![]
                }
                ToolbarClickEvent::None => {
                    vec![]
                }
                ToolbarClickEvent::Dropdown(row) => TOOLBAR_ENTRIES.with(|entries| {
                    let items = &entries[row].items;
                    items
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

        // Selected index should depend on app state
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
    pub fn update_view(&mut self, _ctx: &NodeContext) {
        // HACK: this is temporary, if not here, update ordering causes bugs
        // will prob not be needed after double bindings?
        let val = CLICK_PROP.with(|p| p.get());
        if val != self.click_ev.get() {
            self.click_ev.set(val);
        }
    }
}
