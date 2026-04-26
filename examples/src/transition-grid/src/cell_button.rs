#![allow(unused_imports)]

use pax_kit::*;

#[pax]
#[file("cell_button.pax")]
pub struct CellButton {
    pub label: Property<String>,
    pub hue: Property<Numeric>,
    pub hits: Property<usize>,
    pub display_hue: Property<Numeric>,
    pub last_hue: Property<Numeric>,
}

impl CellButton {
    pub fn handle_mount(&mut self, _ctx: &NodeContext) {
        let hue = self.hue.get();
        self.display_hue.set(hue);
        self.last_hue.set(hue);
    }

    pub fn handle_pre_render(&mut self, _ctx: &NodeContext) {
        let target = self.hue.get();
        if target != self.last_hue.get() {
            self.display_hue.ease_to(target, 24, EasingCurve::OutQuad);
            self.last_hue.set(target);
        }
    }
}
