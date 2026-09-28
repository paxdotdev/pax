use pax_kit::*;

#[pax]
#[main]
#[file("lib.pax")]
pub struct Example {
    pub note: Property<String>,
    pub paragraph: Property<String>,
    pub scroll_y: Property<f64>,
}

impl Example {
    pub fn on_mount(&mut self, _ctx: &NodeContext) {
        self.note.set("Edit this note".into());
        self.paragraph.set("First line\nSecond line\nThird line".into());
    }

    pub fn top(&mut self, _ctx: &NodeContext, _event: Event<ButtonClick>) {
        self.scroll_y.set(0.0);
    }

    pub fn bottom(&mut self, _ctx: &NodeContext, _event: Event<ButtonClick>) {
        self.scroll_y.set(6000.0);
    }
}
