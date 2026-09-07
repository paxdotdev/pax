use pax_kit::*;

#[pax]
#[main]
#[file("lib.pax")]
pub struct Example {
    pub items: Property<Vec<i64>>,
    pub index: Property<usize>,
}

impl Example {
    pub fn on_mount(&mut self, _ctx: &NodeContext) {
        self.items.set(vec![10, 20]);
    }

    pub fn next(&mut self, _ctx: &NodeContext, _event: Event<Click>) {
        // Deliberately change only the index to exercise dependency invalidation.
        self.index.set((self.index.get() + 1) % 2);
    }
}
