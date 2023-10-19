#![allow(unused_imports)]

pub mod website_desktop;
pub mod website_mobile;

use pax_lang::api::*;
use pax_lang::*;
use pax_std::components::Stacker;
use pax_std::primitives::{Frame, Group, Image, Rectangle, Text};
use pax_std::types::{Color, Fill, LinearGradient, StackerDirection};

use crate::website_desktop::WebsiteDesktop;
use crate::website_mobile::WebsiteMobile;

#[derive(Pax)]
#[main]
#[inlined(
for i in 0..5 {
<Stacker cells=10 direction=StackerDirection::Vertical x={(i*50)px} width=50px>
<Rectangle fill={Fill::Solid(Color::rgb(i * 0.2, 0.2, 0.5))} />
</Stacker>
}

)]
pub struct Example {
    pub container_width: Property<f64>,
}

impl Example {
    pub fn handle_did_mount(&mut self, ctx: RuntimeContext) {
        self.container_width.set(ctx.bounds_parent.0);
    }

    pub fn handle_will_render(&mut self, ctx: RuntimeContext) {
        self.container_width.set(ctx.bounds_parent.0);
    }
}
