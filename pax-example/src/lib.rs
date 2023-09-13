#![allow(unused_imports)]

pub mod website_desktop;
pub mod website_mobile;

use pax_lang::*;
use pax_lang::api::*;
use pax_std::primitives::{Frame, Group, Rectangle, Text, Image};
use pax_std::types::{Color, Fill, LinearGradient, StackerDirection};
use pax_std::components::{Stacker};

use crate::website_desktop::WebsiteDesktop;
use crate::website_mobile::WebsiteMobile;

#[derive(Pax)]
#[main]
#[inlined(
    <Frame width=100% height=100% @did_mount=handle_did_mount @will_render=handle_will_render >
     if container_width > 800.0  {
        <WebsiteDesktop />
    }
    if container_width == 800.0 || container_width < 800.0 {
        <WebsiteMobile />
    }
    </Frame>
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