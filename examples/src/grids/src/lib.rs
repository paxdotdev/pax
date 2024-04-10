use pax_engine::api::*;
use pax_engine::*;
use pax_std::components::Stacker;
use pax_std::primitives::*;
use pax_std::types::StackerDirection;

use std::sync::atomic::AtomicI32;

#[pax]
#[main]
#[file("grids.pax")]
pub struct Grids {
    pub eased_value: Property<f64>,
}

impl Grids {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        self.eased_value.ease_to(70.0, 70, EasingCurve::InOutBack);
        self.eased_value
            .ease_to_later(0.0, 200, EasingCurve::InQuad);
        self.eased_value
            .ease_to_later(50.0, 200, EasingCurve::InQuad);
    }

    pub fn tick(&mut self, ctx: &NodeContext) {
        static TIME: AtomicI32 = AtomicI32::new(0);
        if TIME.fetch_add(1, std::sync::atomic::Ordering::Relaxed) == 100 {
            log::debug!("fast to 0.0");
            self.eased_value.ease_to(0.0, 10, EasingCurve::Linear);
        }
    }
}
