#![allow(unused_imports)]

use pax_kit::*;

#[pax]
#[main]
#[file("lib.pax")]
pub struct Example {
    pub selected: Property<usize>,
    pub out_of_range_index: Property<usize>,
    pub duplicate_a: Property<usize>,
    pub duplicate_b: Property<usize>,
    pub pin_first: Property<bool>,
    pub pinned_indices: Property<Vec<usize>>,
    pub status: Property<String>,
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
#[file("out_of_range_deal.pax")]
pub struct OutOfRangeDeal {
    pub index: Property<usize>,
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
        self.out_of_range_index.set(8);
        self.duplicate_a.set(0);
        self.duplicate_b.set(0);
        self.pin_first.set(true);
        self.pinned_indices.set(vec![0, 1]);
        self.sources.set(example_sources());
        self.refresh_status();
    }

    pub fn next_selected(&mut self, _ctx: &NodeContext, _event: Event<Click>) {
        self.selected.set((self.selected.get() + 1) % 2);
        self.refresh_status();
    }

    pub fn next_out_of_range(&mut self, _ctx: &NodeContext, _event: Event<Click>) {
        self.out_of_range_index
            .set(if self.out_of_range_index.get() == 8 {
                9
            } else {
                8
            });
        self.refresh_status();
    }

    pub fn toggle_duplicate(&mut self, _ctx: &NodeContext, _event: Event<Click>) {
        if self.duplicate_b.get() == self.duplicate_a.get() {
            self.duplicate_b.set(1);
        } else {
            self.duplicate_b.set(self.duplicate_a.get());
        }
        self.refresh_status();
    }

    pub fn toggle_conditional(&mut self, _ctx: &NodeContext, _event: Event<Click>) {
        self.pin_first.set(!self.pin_first.get());
        self.refresh_status();
    }

    pub fn rotate_repeated(&mut self, _ctx: &NodeContext, _event: Event<Click>) {
        let next = match self.pinned_indices.get().as_slice() {
            [0, 1] => vec![1, 2],
            [1, 2] => vec![2, 0],
            _ => vec![0, 1],
        };
        self.pinned_indices.set(next);
        self.refresh_status();
    }

    fn refresh_status(&mut self) {
        self.status.set(format!(
            "selected={} | out-of-range probe={} | duplicate=({}, {}) | conditional={} | repeated={:?}",
            self.selected.get(),
            self.out_of_range_index.get(),
            self.duplicate_a.get(),
            self.duplicate_b.get(),
            if self.pin_first.get() { "pinned" } else { "open" },
            self.pinned_indices.get()
        ));
    }
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
            label: "out_of_range_deal.pax".to_string(),
            language: "pax".to_string(),
            code: include_str!("out_of_range_deal.pax").to_string(),
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
