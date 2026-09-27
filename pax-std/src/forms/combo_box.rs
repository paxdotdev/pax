#![allow(unused)]
use std::cell::Cell;
use std::rc::Rc;

use crate::{EventBlocker, Group, Path, Rectangle, Scroller, Stacker, Text};
use crate::{TextStyle, Textbox};
use pax_engine::api::{Click, Event, Store, Stroke};
use pax_engine::api::{Color, Property};
use pax_engine::*;
use pax_runtime::api::NodeContext;

/// A text-filtered list control for selecting one item, with optional "new item" behavior.
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
            <Stacker gutter=1px height={(Math::len(self._filtered_options)*100/5)%}>
                for option_data in self._filtered_options key option_data.key {
                    <ComboBoxListItem style=style background=background data={option_data} @new_item=self.dispatch_new_item/>
                }
            </Stacker>
            <Rectangle fill={stroke.paint} height={(Math::len(self._filtered_options)*100/5)%}/>
        </Scroller>
    }
    if self.text != "" && self.selected != None && !_options_visible {
        <Group  @click=self.remove_index x=100% width=30px>
            <EventBlocker/>
            <Path class="x_symbol" x=50% y=50% width=15px height=15px/>
        </Group>
    }
    <Textbox
        @click=on_click
        text=bind:text
        style=style
        background=background
        stroke=stroke
        corner_radius=corner_radius
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
            color: TRANSPARENT,
            width: 0
        },
        fill: rgb(200, 200, 200)
    }
}

)]
pub struct ComboBox {
    /// Text currently shown in the input.
    pub text: Property<String>,
    /// Index of the last selected option, or `None`. Editing `text` filters the
    /// list; clicking a row commits its original index in `options`.
    pub selected: Property<Option<usize>>,
    /// Available option labels.
    pub options: Property<Vec<String>>,
    /// Behavior when the typed text does not match any option.
    pub new_item: Property<NewItem>,

    /// Textbox/list background color.
    pub background: Property<Color>,
    /// Textbox/list stroke.
    pub stroke: Property<Stroke>,
    /// Text style for the textbox and list items.
    pub style: Property<TextStyle>,
    /// Textbox corner radius, in pixels.
    pub corner_radius: Property<f64>,

    // Private filtered list backing the generated dropdown.
    pub _filtered_options: Property<Vec<ListItemData>>,
    // Private visibility flag for the generated dropdown.
    pub _options_visible: Property<bool>,

    // Private listener used to mirror `selected` into `text`.
    pub _selected_listener: Property<bool>,
}

/// Behavior when typed combo-box text does not match an existing option.
#[pax]
#[engine_import_path("pax_engine")]
pub enum NewItem {
    /// Show "No items found" and do not allow adding a new item.
    #[default]
    Disallow,
    /// Allows unmatched text without showing a no-results row.
    AllowInvalid,
    /// Shows custom text when there are no matches; clicking it triggers the `@new_item` event.
    Text(String),
}

const ZERO_WIDTH_SPACE: &str = "\u{200B}";

struct SelectedIndProp(Property<Option<usize>>);
impl Store for SelectedIndProp {}

impl ComboBox {
    // Binds filtered options and selected-text synchronization.
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        ctx.provide_store(SelectedIndProp(self.selected.clone()))
            .expect("ComboBox is mounting");
        self.bind_listeners();
    }

    fn bind_listeners(&mut self) {
        let options = self.options.clone();
        let text = self.text.clone();
        let new_item = self.new_item.clone();
        let deps = [options.untyped(), text.untyped(), new_item.untyped()];
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
                                key: i,
                                text: v.clone(),
                                event: ComboBoxItemClickEvent::SelectIndex(i),
                            })
                            .collect();
                        filtered_options.sort_by_key(|v| !v.text.starts_with(text));
                        if filtered_options.is_empty() {
                            match new_item.get() {
                                NewItem::Disallow => filtered_options.push(ListItemData {
                                    key: options.len(),
                                    text: String::from("No Results Found"),
                                    event: ComboBoxItemClickEvent::None,
                                }),
                                NewItem::Text(text) => filtered_options.push(ListItemData {
                                    key: options.len(),
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
        // Changing the no-match policy must not reset an in-progress query.
        let deps = [selected.untyped(), options.untyped()];

        let last = Rc::new(Cell::new(None));
        self._selected_listener.replace_with(Property::computed(
            move || {
                // Remove the rows before changing their filter source, so descendants
                // cannot evaluate bindings to indices removed by selection.
                options_visible.set(false);
                let requested = selected.get();
                let selected_value =
                    options.read(|options| requested.and_then(|index| options.get(index).cloned()));
                let current = requested.filter(|_| selected_value.is_some());
                if current != requested {
                    selected.set(current);
                }
                let new_value = selected_value.unwrap_or_default();
                if current.is_some()
                    || requested.is_some()
                    || last.get().is_some()
                    || !matches!(new_item_behavior.get(), NewItem::AllowInvalid)
                {
                    text.set(new_value);
                }
                last.set(current);
                true
            },
            &deps,
        ));
    }

    // Opens the option list.
    pub fn on_click(&mut self, ctx: &NodeContext, _event: Event<Click>) {
        self._options_visible.set(true);
    }

    // Clears the current selected item.
    pub fn remove_index(&mut self, ctx: &NodeContext, event: Event<Click>) {
        self.selected.set(None);
        self.text.set("".to_string());
    }

    // Forces selected-text synchronization during pre-render.
    pub fn update(&mut self, ctx: &NodeContext) {
        self._selected_listener.get();
    }

    // Emits the combo box's `new_item` event.
    pub fn dispatch_new_item(&mut self, ctx: &NodeContext) {
        ctx.dispatch_event("new_item").unwrap();
    }
}

// Internal list row used by `ComboBox`.
#[pax]
#[engine_import_path("pax_engine")]
#[inlined(
    <Text x=3px text={data.text} width={100%-3px} height=100% style=style/>
    <Rectangle fill=background/>

    @settings {
        @click: on_click
    }
)]
pub struct ComboBoxListItem {
    // Row payload.
    pub data: Property<ListItemData>,
    // Row background color.
    pub background: Property<Color>,
    // Row text style.
    pub style: Property<TextStyle>,
}

impl ComboBoxListItem {
    // Applies this row's action to the parent combo box.
    pub fn on_click(&mut self, ctx: &NodeContext, _event: Event<Click>) {
        match self.data.get().event {
            ComboBoxItemClickEvent::None => (),
            ComboBoxItemClickEvent::SelectIndex(index) => {
                let _ = ctx.with_store(|SelectedIndProp(selected): &mut SelectedIndProp| {
                    selected.set(Some(index));
                });
            }
            ComboBoxItemClickEvent::NewItem => {
                ctx.dispatch_event("new_item");
            }
        }
    }
}

// Internal row payload used by `ComboBoxListItem`.
#[pax]
#[engine_import_path("pax_engine")]
pub struct ListItemData {
    // Source index keeps filtered/reordered rows attached to their own payload.
    pub key: usize,
    // Row label shown in the dropdown.
    pub text: String,
    // Action dispatched when this row is selected.
    pub event: ComboBoxItemClickEvent,
}

// Internal row action for `ComboBoxListItem`.
#[pax]
#[engine_import_path("pax_engine")]
pub enum ComboBoxItemClickEvent {
    // Disabled row.
    #[default]
    None,
    // Selects an existing option index.
    SelectIndex(usize),
    // Dispatches `new_item`.
    NewItem,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn combo(options: &[&str]) -> ComboBox {
        let mut combo = ComboBox::default();
        combo
            .options
            .set(options.iter().map(|s| s.to_string()).collect());
        combo.bind_listeners();
        combo._selected_listener.get();
        combo
    }

    #[test]
    fn filtering_preserves_source_indices_and_ranks_prefixes_first() {
        let combo = combo(&["zBeta", "Beta", "Alpha"]);
        combo.text.set("Beta".into());
        let rows = combo._filtered_options.get();
        assert_eq!(
            rows.iter().map(|row| row.text.as_str()).collect::<Vec<_>>(),
            vec!["Beta", "zBeta"]
        );
        assert!(matches!(
            rows[0].event,
            ComboBoxItemClickEvent::SelectIndex(1)
        ));
        assert!(matches!(
            rows[1].event,
            ComboBoxItemClickEvent::SelectIndex(0)
        ));
        assert_eq!(
            rows.iter().map(|row| row.key).collect::<Vec<_>>(),
            vec![1, 0]
        );
    }

    #[test]
    fn selection_closes_the_list_and_options_changes_update_the_label() {
        let combo = combo(&["Alpha", "Beta"]);
        combo._options_visible.set(true);
        combo.selected.set(Some(1));
        combo._selected_listener.get();
        assert_eq!(combo.text.get(), "Beta");
        assert!(!combo._options_visible.get());
        combo.options.set(vec!["Alpha".into(), "Renamed".into()]);
        combo._selected_listener.get();
        assert_eq!(combo.text.get(), "Renamed");
    }

    #[test]
    fn missing_and_out_of_range_selections_clear_without_panicking() {
        let combo = combo(&["Alpha", "Beta"]);
        for index in [2, usize::MAX] {
            combo.selected.set(Some(index));
            combo._selected_listener.get();
            assert_eq!(combo.selected.get(), None);
            assert_eq!(combo.text.get(), "");
        }
        combo.selected.set(Some(1));
        combo._selected_listener.get();
        combo.options.set(Vec::new());
        combo._selected_listener.get();
        assert_eq!(combo.selected.get(), None);
        assert_eq!(combo.text.get(), "");
    }

    #[test]
    fn changing_no_match_behavior_refreshes_existing_filter() {
        let combo = combo(&["Alpha"]);
        combo.text.set("missing".into());
        assert!(matches!(
            combo._filtered_options.get()[0].event,
            ComboBoxItemClickEvent::None
        ));
        combo.new_item.set(NewItem::Text("Create missing".into()));
        combo._selected_listener.get();
        assert_eq!(combo.text.get(), "missing");
        let rows = combo._filtered_options.get();
        assert_eq!(rows[0].text, "Create missing");
        assert!(matches!(rows[0].event, ComboBoxItemClickEvent::NewItem));
        combo.new_item.set(NewItem::AllowInvalid);
        combo._selected_listener.get();
        assert_eq!(combo.text.get(), "missing");
        assert!(combo._filtered_options.get().is_empty());
    }

    #[test]
    fn duplicate_labels_keep_distinct_option_keys() {
        let combo = combo(&["Same", "Same"]);
        let rows = combo._filtered_options.get();
        assert_eq!(
            rows.iter().map(|row| row.key).collect::<Vec<_>>(),
            vec![0, 1]
        );
        combo.text.set("missing".into());
        assert_eq!(combo._filtered_options.get()[0].key, 2);
    }

    #[test]
    fn allow_invalid_preserves_free_text_but_mirrors_committed_labels() {
        let mut combo = ComboBox::default();
        combo.options.set(vec!["Alpha".into()]);
        combo.new_item.set(NewItem::AllowInvalid);
        combo.text.set("Free text".into());
        combo.bind_listeners();
        combo._selected_listener.get();
        assert_eq!(combo.text.get(), "Free text");
        combo.selected.set(Some(0));
        combo._selected_listener.get();
        assert_eq!(combo.text.get(), "Alpha");
        combo.options.set(vec!["Renamed".into()]);
        combo._selected_listener.get();
        assert_eq!(combo.text.get(), "Renamed");
        combo.options.set(vec![]);
        combo._selected_listener.get();
        assert_eq!(combo.selected.get(), None);
        assert_eq!(combo.text.get(), "");
    }
}
