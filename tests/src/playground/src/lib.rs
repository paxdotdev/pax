#![allow(unused_imports)]
use pax_engine::api::*;
use pax_engine::*;
use pax_std::*;

#[pax]
#[main]
#[file("lib.pax")]
pub struct Example {
    pub ticks: Property<u8>,
    pub num_clicks: Property<usize>,
    pub frame_clip: Property<bool>,
    pub data: Property<Vec<u8>>,
}

impl Example {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        let mut data = palette([255, 0, 0]);
        self.data.set(data.to_vec());
    }
    pub fn handle_pre_render(&mut self, ctx: &NodeContext) {
        let old_ticks = self.ticks.get();
        let mut data = palette([255, old_ticks, 0]);
        self.data.set(data.to_vec());
        self.ticks.set((old_ticks + 1) % 255);
    }
}

#[rustfmt::skip]
fn palette(c: [u8; 3]) -> [u8; 2 * 3 * 4] {
    [
        255, 255, 255, 255,       /**/ 255, 255, 255, 255,
        255/2, 255/2, 255/2, 255, /**/ c[0], c[1], c[2], 255,
        0, 0, 0, 255,             /**/ 0,0, 0, 255
    ]
}
