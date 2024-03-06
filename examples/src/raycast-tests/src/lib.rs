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
    pub hit_outer: Property<String>,
    pub hit_inner: Property<String>,
}

impl Example {
    pub fn handle_mount(&mut self, ctx: &NodeContext) {}

    pub fn frame1(&mut self, ctx: &NodeContext, args: Event<MouseMove>) {
        self.hit_outer.set(format!("hit outer: frame {}", 1));
    }

    pub fn frame2(&mut self, ctx: &NodeContext, args: Event<MouseMove>) {
        self.hit_outer.set(format!("hit outer: frame {}", 2));
    }

    pub fn frame1rect1(&mut self, ctx: &NodeContext, args: Event<MouseMove>) {
        self.hit_inner.set(format!("hit inner: rect {}", 1));
    }

    pub fn frame1rect2(&mut self, ctx: &NodeContext, args: Event<MouseMove>) {
        self.hit_inner.set(format!("hit inner: rect {}", 2));
    }

    pub fn frame2rect1(&mut self, ctx: &NodeContext, args: Event<MouseMove>) {
        self.hit_inner.set(format!("hit inner: rect {}", 1));
    }
}
