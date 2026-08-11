#![allow(unused_imports)]

use pax_kit::*;

const COMPACT_BREAKPOINT_PX: f64 = 760.0;

#[pax]
#[file("hero_section.pax")]
pub struct HeroSection {
    pub compact: Property<bool>,
    pub copy_x_px: Property<f64>,
    pub copy_width_px: Property<f64>,
    pub map_x_px: Property<f64>,
    pub map_width_px: Property<f64>,
}

impl HeroSection {
    pub fn handle_mount(&mut self, ctx: &NodeContext) {
        self.sync_layout(ctx);
    }

    pub fn handle_pre_render(&mut self, ctx: &NodeContext) {
        self.sync_layout(ctx);
    }

    fn sync_layout(&mut self, ctx: &NodeContext) {
        let width = ctx.bounds_self.get().0;
        let compact = width < COMPACT_BREAKPOINT_PX;
        self.compact.set_if_neq(compact);

        if compact {
            self.copy_x_px.set_if_neq(20.0);
            self.copy_width_px.set_if_neq((width - 40.0).max(0.0));
            self.map_x_px.set_if_neq(20.0);
            self.map_width_px.set_if_neq((width - 40.0).max(0.0));
        } else {
            self.copy_x_px.set_if_neq(width * 0.06);
            self.copy_width_px.set_if_neq(width * 0.53);
            self.map_x_px.set_if_neq(width * 0.64);
            self.map_width_px.set_if_neq(width * 0.31);
        }
    }
}
