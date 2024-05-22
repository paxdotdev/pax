use pax_engine::api::*;
use pax_engine::*;
use pax_std::components::Stacker;
use pax_std::components::*;
use pax_std::primitives::*;
use pax_std::types::text::*;
use pax_std::types::*;
use std::cmp::Ordering;
use std::iter;

#[pax]
#[file("components/tabs.pax")]
pub struct Tabs {
    pub names: Property<Vec<String>>,
    pub selected: Property<usize>,
    pub color: Property<Color>,

    // private
    pub slot_count: Property<usize>,
    pub names_filled: Property<Vec<String>>,
}

impl Tabs {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        let slot_count = ctx.slot_children_count.clone();
        let deps = [slot_count.untyped()];
        self.slot_count
            .replace_with(Property::computed(move || slot_count.get(), &deps));
        let slot_count = ctx.slot_children_count.clone();
        let names = self.names.clone();
        let deps = [slot_count.untyped(), names.untyped()];
        self.names_filled.replace_with(Property::computed(
            move || {
                let names = names.get();
                let mut names_filled = vec![];
                for i in 0..slot_count.get() {
                    names_filled.push(
                        names
                            .get(i)
                            .map(|s| s.as_str())
                            .unwrap_or("[no name]")
                            .to_owned(),
                    );
                }
                names_filled
            },
            &deps,
        ));
    }

    pub fn on_click(&mut self, ctx: &NodeContext, event: Event<Click>) {
        let bounds = ctx.bounds_self.get();
        let parts = self.slot_count.get();
        let x = event.mouse.x;
        let id = (x * parts as f64 / bounds.0) as usize;
        self.selected.set(id);
    }
}
