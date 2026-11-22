use crate::{circuit_board::CircuitBoard, modularity_rain::ModularityRain, SiteTheme};
use pax_kit::*;

#[pax]
#[file("modularity_section.pax")]
pub struct ModularitySection {
    pub light_mode: Property<bool>,
    pub compact: Property<bool>,
    pub content_height: Property<f64>,
    pub _height_bound: Property<bool>,
}

impl ModularitySection {
    pub fn mount(&mut self, ctx: &NodeContext) {
        self._height_bound.set(false);
        crate::section_layout::bind_compact(ctx, &self.compact, 1000.0);
    }

    pub fn measure(&mut self, ctx: &NodeContext) {
        if !self._height_bound.get() {
            self._height_bound.set(crate::section_layout::bind_height(
                ctx,
                "modularity_content",
                &self.content_height,
            ));
        }
    }
}
