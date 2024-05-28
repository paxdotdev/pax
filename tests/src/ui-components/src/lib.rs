#![allow(unused_imports)]

pub use pax_component_library::ConfirmationDialog;
pub use pax_component_library::PaxDropdown;
pub use pax_component_library::PaxRadioSet;
pub use pax_component_library::PaxSlider;
pub use pax_component_library::Resizable;
pub use pax_component_library::Table;
pub use pax_component_library::Tabs;
pub use pax_component_library::Toast;

use pax_component_library::table::Cell;
use pax_engine::api::*;
use pax_engine::*;
use pax_std::components::Stacker;
use pax_std::components::*;
use pax_std::primitives::*;
use pax_std::types::text::*;
use pax_std::types::*;

#[pax]
#[main]
#[file("lib.pax")]
#[custom(Default)]
pub struct Example {
    pub selected: Property<u32>,
    pub dialog_open: Property<bool>,
    pub message: Property<String>,
    pub signal: Property<bool>,
    pub headers: Property<Vec<Cell>>,
    pub rows: Property<Vec<Vec<Cell>>>,
    pub row_colors: Property<Vec<Color>>,
}

impl Default for Example {
    fn default() -> Self {
        Self {
            message: Property::default(),
            selected: Property::new(1),
            dialog_open: Property::new(false),
            signal: Property::new(false),
            headers: Property::new(vec![
                Cell {
                    text: "And a 1".to_owned(),
                    text_align: TextAlignHorizontal::Center,
                },
                Cell {
                    text: "And a 2".to_owned(),
                    text_align: TextAlignHorizontal::Center,
                },
                Cell {
                    text: "And a 3".to_owned(),
                    text_align: TextAlignHorizontal::Right,
                },
            ]),
            rows: Property::new(vec![
                vec![
                    Cell {
                        text: "first".to_string(),
                        text_align: TextAlignHorizontal::Left,
                    },
                    Cell {
                        text: "second".to_string(),
                        text_align: TextAlignHorizontal::Left,
                    },
                    Cell {
                        text: "third".to_string(),
                        text_align: TextAlignHorizontal::Right,
                    },
                ],
                vec![
                    Cell {
                        text: "middle first".to_string(),
                        text_align: TextAlignHorizontal::Left,
                    },
                    Cell {
                        text: "middle second".to_string(),
                        text_align: TextAlignHorizontal::Center,
                    },
                    Cell {
                        text: "middle third".to_string(),
                        text_align: TextAlignHorizontal::Right,
                    },
                ],
                vec![
                    Cell {
                        text: "low first".to_string(),
                        text_align: TextAlignHorizontal::Left,
                    },
                    Cell {
                        text: "low second".to_string(),
                        text_align: TextAlignHorizontal::Left,
                    },
                    Cell {
                        text: "low third".to_string(),
                        text_align: TextAlignHorizontal::Right,
                    },
                ],
            ]),
            row_colors: Property::new(vec![Color::RED, Color::YELLOW]),
        }
    }
}

impl Example {
    pub fn on_click(&mut self, ctx: &NodeContext, event: Event<Click>) {
        self.selected.set(2);
        self.dialog_open.set(true);
    }
    pub fn on_left_side_click(&mut self, ctx: &NodeContext, event: Event<Click>) {
        self.message
            .set(format!("this is a message! mouse x-pos: {}", event.mouse.x));
    }
}
