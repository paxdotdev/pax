use pax_engine::*;
use pax_engine::api::*;

use pax_std::primitives::Ellipse;

#[pax]
#[file("ball.pax")]
pub struct Ball {
    pub magnitude: Property<Numeric>,
    pub effective_diameter: Property<Numeric>,
}

impl Ball {
    pub fn handle_mount(&mut self, ctx: &NodeContext) {

        let steady = Numeric::from(1.5 * self.magnitude.get());
        let lower = Numeric::from(0.75 * self.magnitude.get());
        let upper = Numeric::from(1.75 * self.magnitude.get());

        //TODO: probably not updating eased values in handle_vtable_updates; need to hook back up
        self.effective_diameter.set(lower);
        self.effective_diameter.ease_to(upper,30, EasingCurve::Linear);
        self.effective_diameter.ease_to_later(steady,30, EasingCurve::Linear);

    }
}
