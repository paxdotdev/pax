#![allow(unused)]
use std::cell::Cell;
use std::rc::Rc;

use crate::{EventBlocker, Group, Path, Rectangle, Scroller, Stacker, Text};
use crate::{TextStyle, Textbox};
use pax_engine::api::{Click, Event, MouseDown, Store, Stroke, TextboxChange};
use pax_engine::api::{Color, Property};
use pax_engine::*;
use pax_runtime::api::NodeContext;

#[pax]
#[engine_import_path("pax_engine")]
#[inlined(
    if _options_visible {
        <Scroller
            anchor_y=0%
            y={100%+1px}
            height=500%
            width={100% + 8px}
            x=1px
            scroll_height={(Math::len(self._filtered_options)*100/5)%}
    		unclippable=true
        >
            <Stacker gutter=1px>
                for option_data in self._filtered_options {
                    <ComboBoxListItem style=style background=background data={option_data} @new_item=self.dispatch_new_item/>
                }
            </Stacker>
            <Rectangle fill={stroke.color}/>
        </Scroller>
    }
    if self.text != "" && self.selected != None && !_options_visible {
        <Group  @click=self.remove_index x=100% width=30px>
            <EventBlocker/>
            <Path class=x_symbol x=50% y=50% width=15px height=15px/>
        </Group>
    }
    <Textbox
        @click=on_click
        text=bind:text
        style=style
        background=background
        stroke=stroke
        border_radius=border_radius
        @textbox_change=self.textbox_change
    />

    @settings {
        @mount: on_mount
        @pre_render: update

    .x_symbol {
        elements: {[
            PathElement::Point(0%, 10%),
            PathElement::Line,
            PathElement::Point(50% - 10%, 50%),
            PathElement::Line,
            PathElement::Point(0%, 100% - 10%),
            PathElement::Line,
            PathElement::Point(10%, 100%),
            PathElement::Line,
            PathElement::Point(50%, 50% + 10%),
            PathElement::Line,
            PathElement::Point(100% - 10%, 100%),
            PathElement::Line,
            PathElement::Point(100%, 100% - 10%),
            PathElement::Line,
            PathElement::Point(50% + 10%, 50%),
            PathElement::Line,
            PathElement::Point(100%, 10%),
            PathElement::Line,
            PathElement::Point(100% - 10%, 0%),
            PathElement::Line,
            PathElement::Point(50%, 50% - 10%),
            PathElement::Line,
            PathElement::Point(10%, 0%),
            PathElement::Close
        ]},
        stroke: {
            width: 0
        },
        fill: rgb(200, 200, 200)
    }
}

)]
pub struct ComboBox {
    // public
    pub text: Property<String>,
    pub selected: Property<Option<usize>>,
    pub options: Property<Vec<String>>,
    pub new_item: Property<NewItem>,

    // styling
    pub background: Property<Color>,
    pub stroke: Property<Stroke>,
    pub style: Property<TextStyle>,
    pub border_radius: Property<f64>,

    //private
    pub _filtered_options: Property<Vec<ListItemData>>,
    pub _options_visible: Property<bool>,

    pub _selected_listener: Property<bool>,
}

#[pax]
#[engine_import_path("pax_engine")]
pub enum NewItem {
    // Show "No items found", and don't allow adding a new item
    #[default]
    Disallow,
    /// Allows for invalid text in the text box on commit, setting selected to None
    AllowInvalid,
    // Custom name for the item shown if no matches, when clicked, triggers the @new_item custom event
    Text(String),
}

const ZERO_WIDTH_SPACE: &str = "\u{200B}";

struct SelectedIndProp(Property<Option<usize>>);
impl Store for SelectedIndProp {}

impl ComboBox {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        ctx.push_local_store(SelectedIndProp(self.selected.clone()));
        let options = self.options.clone();
        let text = self.text.clone();
        let deps = [options.untyped(), text.untyped()];
        let new_item = self.new_item.clone();
        self._filtered_options.replace_with(Property::computed(
            move || {
                options.read(|options| {
                    text.read(|text| {
                        let mut options_sorted: Vec<(usize, &String)> =
                            options.iter().enumerate().collect();
                        options_sorted.sort_by_key(|&(_, v)| v);
                        let mut filtered_options: Vec<_> = options_sorted
                            .into_iter()
                            .filter(|(_, t)| t.contains(text))
                            .map(|(i, v)| ListItemData {
                                text: v.clone(),
                                event: ComboBoxItemClickEvent::SelectIndex(i),
                            })
                            .collect();
                        filtered_options.sort_by_key(|v| v.text.starts_with(text));
                        if filtered_options.is_empty() {
                            match new_item.get() {
                                NewItem::Disallow => filtered_options.push(ListItemData {
                                    text: String::from("No Results Found"),
                                    event: ComboBoxItemClickEvent::None,
                                }),
                                NewItem::Text(text) => filtered_options.push(ListItemData {
                                    text,
                                    event: ComboBoxItemClickEvent::NewItem,
                                }),
                                NewItem::AllowInvalid => (),
                            }
                        }
                        filtered_options
                    })
                })
            },
            &deps,
        ));
        let text = self.text.clone();
        let options = self.options.clone();
        let options_visible = self._options_visible.clone();
        let new_item_behavior = self.new_item.clone();

        let selected = self.selected.clone();
        let deps = [selected.untyped()];

        let last = Rc::new(Cell::new(None));
        self._selected_listener.replace_with(Property::computed(
            move || {
                let selected = selected.get();
                let new_value = if let Some(selected) = selected {
                    options.read(|options| {
                        let index = selected.clamp(0, options.len());
                        options[index].clone()
                    })
                } else {
                    "".to_string()
                };
                match new_item_behavior.get() {
                    NewItem::AllowInvalid => {
                        if last.get() != selected || text.get().is_empty() {
                            text.set(new_value)
                        }
                    }
                    _ => text.set(new_value),
                }
                options_visible.set(false);
                last.set(selected);
                true
            },
            &deps,
        ));
    }

    pub fn on_click(&mut self, ctx: &NodeContext, _event: Event<Click>) {
        self._options_visible.set(true);
    }

    pub fn textbox_change(&mut self, ctx: &NodeContext, _event: Event<TextboxChange>) {
        self.selected.set(self.selected.get());
    }

    pub fn remove_index(&mut self, ctx: &NodeContext, event: Event<Click>) {
        self.selected.set(None);
        self.text.set("".to_string());
    }

    pub fn update(&mut self, ctx: &NodeContext) {
        self._selected_listener.get();
    }

    pub fn dispatch_new_item(&mut self, ctx: &NodeContext) {
        ctx.dispatch_event("new_item").unwrap();
    }
}

#[pax]
#[engine_import_path("pax_engine")]
#[inlined(
    <Text x=3px text={data.text} width={100%-3px} height=100% style=style/>
    <Rectangle fill=background/>

    @settings {
        @mouse_down: on_click
    }
)]
pub struct ComboBoxListItem {
    pub data: Property<ListItemData>,
    pub background: Property<Color>,
    pub style: Property<TextStyle>,
}

impl ComboBoxListItem {
    pub fn on_click(&mut self, ctx: &NodeContext, _event: Event<MouseDown>) {
        match self.data.get().event {
            ComboBoxItemClickEvent::None => (),
            ComboBoxItemClickEvent::SelectIndex(index) => {
                let _ = ctx.peek_local_store(|SelectedIndProp(selected): &mut SelectedIndProp| {
                    selected.set(Some(index));
                });
            }
            ComboBoxItemClickEvent::NewItem => {
                ctx.dispatch_event("new_item");
            }
        }
    }
}

#[pax]
#[engine_import_path("pax_engine")]
pub struct ListItemData {
    pub text: String,
    pub event: ComboBoxItemClickEvent,
}

#[pax]
#[engine_import_path("pax_engine")]
pub enum ComboBoxItemClickEvent {
    #[default]
    None,
    SelectIndex(usize),
    NewItem,
}
