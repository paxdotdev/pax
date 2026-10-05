use pax_kit::*;

// Exercise the compatibility shorthand as a reusable dependency component.
#[pax]
#[main]
#[file("src/counter.pax")]
pub struct Counter {
    pub count: Property<usize>,
    pub mounted: Property<bool>,
}

impl Counter {
    pub fn on_mount(&mut self, _: &NodeContext) {
        self.mounted.set(true);
    }

    pub fn increment(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.count.update(|value| *value += 1);
    }
}
