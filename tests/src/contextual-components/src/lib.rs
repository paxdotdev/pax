#![allow(unused_imports)]

use pax_engine::api::*;
use pax_engine::*;
use pax_std::*;
pub mod contextual;
pub use contextual::{ContextualChild, ContextualParent};

#[pax]
#[main]
#[file("lib.pax")]
pub struct Example {
    pub num: Property<usize>,
    pub text: Property<String>,
}

impl Example {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        self.num.set(1);
        self.text.set("hello".to_string());
    }

    pub fn click(&mut self, ctx: &NodeContext, event: Event<Click>) {
        if event.mouse.x > 100.0 {
            log::info!("incrementing num");
            self.num.set(self.num.get() + 1);
        } else {
            log::info!("adding to text");
            self.text.set(self.text.get() + "O");
        }
    }
}
