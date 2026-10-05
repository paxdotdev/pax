use pax_kit::*;
#[allow(unused_imports)]
use layout_controls::Counter;

#[pax]
#[file("src/ui/app.pax")]
pub struct App {
    pub count: Property<usize>,
}

impl App {
    pub fn increment(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.count.update(|value| *value += 1);
    }
}
