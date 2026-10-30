use pax_kit::*;

pax_kit::runnable! { root: crate::Example }

#[pax]
#[file("lib.pax")]
pub struct Example {
    pub count: Property<usize>,
    pub note: Property<String>,
    pub notifications: Property<usize>,
    pub dialog_open: Property<bool>,
}

impl Example {
    pub fn mount(&mut self, _ctx: &NodeContext) {
        self.note.set("Edit this note".into());
    }

    pub fn increment(&mut self, _ctx: &NodeContext, _event: Event<ButtonClick>) {
        self.count.set(self.count.get() + 1);
    }

    pub fn notify(&mut self, _ctx: &NodeContext, _event: Event<ButtonClick>) {
        self.notifications.set(self.notifications.get() + 1);
    }

    pub fn confirm(&mut self, _ctx: &NodeContext, _event: Event<ButtonClick>) {
        self.dialog_open.set(true);
    }
}

#[pax]
#[file("notice.pax")]
pub struct Notice {
    #[pax(content)]
    pub message: Property<String>,
}
