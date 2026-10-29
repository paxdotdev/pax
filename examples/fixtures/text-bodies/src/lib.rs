use pax_kit::*;

pax_kit::runnable! { root: crate::Example }

#[pax]
#[file("lib.pax")]
pub struct Example {
    pub count: Property<usize>,
    pub note: Property<String>,
}

impl Example {
    pub fn mount(&mut self, _ctx: &NodeContext) {
        self.note.set("Edit this note".into());
    }

    pub fn increment(&mut self, _ctx: &NodeContext, _event: Event<ButtonClick>) {
        self.count.set(self.count.get() + 1);
    }
}

#[pax]
#[file("notice.pax")]
pub struct Notice {
    #[pax(content)]
    pub message: Property<String>,
}
