#![allow(unused_imports)]

use pax_kit::*;

#[pax]
#[main]
#[file("lib.pax")]
pub struct Example {
    pub selected: Property<usize>,
    pub duplicate_a: Property<usize>,
    pub duplicate_b: Property<usize>,
    pub pin_first: Property<bool>,
    pub pinned_indices: Property<Vec<usize>>,
    pub repeated_offset: Property<usize>,
    pub repeated_color_0: Property<Color>,
    pub repeated_color_1: Property<Color>,
    pub repeated_color_2: Property<Color>,
    pub repeated_color_3: Property<Color>,
    pub repeated_color_4: Property<Color>,
    pub repeated_color_5: Property<Color>,
    pub repeated_color_6: Property<Color>,
    pub sources: Property<Vec<ExampleSource>>,
}

#[pax]
#[file("gem_tile.pax")]
pub struct GemTile {
    pub index: Property<usize>,
    pub label: Property<String>,
    pub color: Property<Color>,
}

#[pax]
#[file("first_three_then_rest.pax")]
pub struct FirstThreeThenRest {}

#[pax]
#[file("dynamic_deal.pax")]
pub struct DynamicDeal {
    pub selected: Property<usize>,
}

#[pax]
#[file("duplicate_deal.pax")]
pub struct DuplicateDeal {
    pub a: Property<usize>,
    pub b: Property<usize>,
}

#[pax]
#[file("conditional_deal.pax")]
pub struct ConditionalDeal {
    pub pin_first: Property<bool>,
}

#[pax]
#[file("repeated_deal.pax")]
pub struct RepeatedDeal {
    pub pinned_indices: Property<Vec<usize>>,
}

impl Example {
    pub fn handle_mount(&mut self, _ctx: &NodeContext) {
        self.selected.set(0);
        self.duplicate_a.set(0);
        self.duplicate_b.set(0);
        self.pin_first.set(true);
        self.pinned_indices.set(vec![0, 1]);
        self.repeated_offset.set(0);
        self.set_repeated_colors(0);
        self.sources.set(example_sources());
    }

    fn set_repeated_colors(&mut self, offset: usize) {
        let colors = repeated_palette();
        self.repeated_color_0
            .set(colors[(offset + 0) % REPEATED_COLOR_COUNT].clone());
        self.repeated_color_1
            .set(colors[(offset + 1) % REPEATED_COLOR_COUNT].clone());
        self.repeated_color_2
            .set(colors[(offset + 2) % REPEATED_COLOR_COUNT].clone());
        self.repeated_color_3
            .set(colors[(offset + 3) % REPEATED_COLOR_COUNT].clone());
        self.repeated_color_4
            .set(colors[(offset + 4) % REPEATED_COLOR_COUNT].clone());
        self.repeated_color_5
            .set(colors[(offset + 5) % REPEATED_COLOR_COUNT].clone());
        self.repeated_color_6
            .set(colors[(offset + 6) % REPEATED_COLOR_COUNT].clone());
    }

    pub fn rotate_repeated(&mut self, _ctx: &NodeContext, _event: Event<Click>) {
        let next = (self.repeated_offset.get() + 1) % REPEATED_COLOR_COUNT;
        self.repeated_offset.set(next);
        self.pinned_indices.set(vec![0, 1]);
        self.set_repeated_colors(next);
    }

    pub fn next_selected(&mut self, _ctx: &NodeContext, _event: Event<Click>) {
        self.selected.set((self.selected.get() + 1) % 2);
    }

    pub fn toggle_duplicate(&mut self, _ctx: &NodeContext, _event: Event<Click>) {
        if self.duplicate_b.get() == self.duplicate_a.get() {
            self.duplicate_b.set(1);
        } else {
            self.duplicate_b.set(self.duplicate_a.get());
        }
    }

    pub fn toggle_conditional(&mut self, _ctx: &NodeContext, _event: Event<Click>) {
        self.pin_first.set(!self.pin_first.get());
    }
}

const REPEATED_COLOR_COUNT: usize = 7;

fn repeated_palette() -> [Color; REPEATED_COLOR_COUNT] {
    [
        gem_color(255, 42, 88),
        gem_color(255, 145, 0),
        gem_color(255, 221, 64),
        gem_color(27, 211, 137),
        gem_color(37, 124, 255),
        gem_color(93, 86, 255),
        gem_color(181, 87, 255),
    ]
}

fn gem_color(r: i32, g: i32, b: i32) -> Color {
    Color::rgb(r.into(), g.into(), b.into())
}

fn example_sources() -> Vec<ExampleSource> {
    vec![
        ExampleSource {
            label: "src/lib.pax".to_string(),
            language: "pax".to_string(),
            code: include_str!("lib.pax").to_string(),
        },
        ExampleSource {
            label: "src/lib.rs".to_string(),
            language: "rust".to_string(),
            code: include_str!("lib.rs").to_string(),
        },
        ExampleSource {
            label: "first_three_then_rest.pax".to_string(),
            language: "pax".to_string(),
            code: include_str!("first_three_then_rest.pax").to_string(),
        },
        ExampleSource {
            label: "dynamic_deal.pax".to_string(),
            language: "pax".to_string(),
            code: include_str!("dynamic_deal.pax").to_string(),
        },
        ExampleSource {
            label: "duplicate_deal.pax".to_string(),
            language: "pax".to_string(),
            code: include_str!("duplicate_deal.pax").to_string(),
        },
        ExampleSource {
            label: "conditional_deal.pax".to_string(),
            language: "pax".to_string(),
            code: include_str!("conditional_deal.pax").to_string(),
        },
        ExampleSource {
            label: "repeated_deal.pax".to_string(),
            language: "pax".to_string(),
            code: include_str!("repeated_deal.pax").to_string(),
        },
        ExampleSource {
            label: "gem_tile.pax".to_string(),
            language: "pax".to_string(),
            code: include_str!("gem_tile.pax").to_string(),
        },
    ]
}
