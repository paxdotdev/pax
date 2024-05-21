#![allow(unused_imports)]

use pax_engine::api::*;
use pax_engine::*;
use pax_std::components::Stacker;
use pax_std::components::*;
use pax_std::primitives::*;
use pax_std::types::text::*;
use pax_std::types::*;

#[pax]
#[main]
#[file("lib.pax")]
pub struct Example {
    pub count: Property<usize>,
    pub image_mode: Property<ImageFit>,
    pub text: Property<String>,
}

impl Example {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        self.count.set(2);
    }

    pub fn increment(&mut self, ctx: &NodeContext, args: Event<Click>) {
        let count = self.count.get();
        let (mode, name) = match count % 5 {
            0 => (ImageFit::Fill, "fill"),
            1 => (ImageFit::FillHorizontal, "fill horiz"),
            2 => (ImageFit::FillVertical, "fill vert"),
            3 => (ImageFit::Fit, "fit"),
            4 => (ImageFit::Stretch, "stretch"),
            _ => (ImageFit::Fill, "none"),
        };
        self.image_mode.set(mode);
        self.text.set(name.to_string());
        self.count.set(count + 1);
    }
}
