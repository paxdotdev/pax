#![allow(unused)]
use crate::{Group, Rectangle, Scroller, Stacker, Text, Path, EventBlocker};
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
                    <ComboBoxListItem style=style background=background data={option_data}/>
                }
            </Stacker>
            <Rectangle fill={stroke.color}/>
        </Scroller>
    }
    if self.text != "" && !_options_visible {
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

const ZERO_WIDTH_SPACE: &str = "\u{200B}";

struct SelectedIndProp(Property<Option<usize>>);
impl Store for SelectedIndProp {}

impl ComboBox {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        ctx.push_local_store(SelectedIndProp(self.selected.clone()));
        let options = self.options.clone();
        let text = self.text.clone();
        let deps = [options.untyped(), text.untyped()];
        self._filtered_options.replace_with(Property::computed(
            move || {
                options.read(|options| {
                    text.read(|text| {
                        let mut options_sorted: Vec<(usize, &String)> = options.iter().enumerate().collect();
                        options_sorted.sort_by_key(|&(_, v)| v);
                        let mut filtered_options: Vec<_> = options_sorted
                            .into_iter()
                            .filter(|(_, t)| t.contains(text))
                            .map(|(i, v)| ListItemData { text: v.clone(), index: Some(i) })
                            .collect();
                        filtered_options.sort_by_key(|v| v.text.starts_with(text));
                        if filtered_options.is_empty() {
                            filtered_options.push(ListItemData { text: String::from("No Results Found"), index: None })
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
        
        let selected = self.selected.clone();
        let deps = [selected.untyped()];

        self._selected_listener.replace_with(Property::computed(move || {
            let selected = selected.get();
            let new_value = if let Some(selected) = selected {
                options.read(|options| {
                    let index = selected.clamp(0, options.len()); 
                    options[index].clone()
                }) 
            } else {
                "".to_string()
            };
            log::debug!("selected set to: {} (ind {:?})", new_value, selected);
            text.set(new_value);
            options_visible.set(false);
            true
        }, &deps));
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
        log::debug!("selecting index: {:?}", self.data.get().index);
        if let Some(index) = self.data.get().index {
            ctx.peek_local_store(|SelectedIndProp(selected): &mut SelectedIndProp| {
                    selected.set(Some(index));
            });
        }
    }
}

#[pax]
#[engine_import_path("pax_engine")]
pub struct ListItemData {
    pub text: String,
    pub index: Option<usize>,
}
