use pax_engine::*;
use pax_engine::api::*;

use pax_std::primitives::Ellipse;

#[pax]
#[file("ball.pax")]
pub struct Ball {
    pub magnitude: Property<Numeric>,
    pub effective_diameter: Property<Numeric>,
    pub effective_hue: Property<Numeric>,
}

impl Ball {
    pub fn handle_mount(&mut self, ctx: &NodeContext) {
        let magnitude = self.magnitude.get().to_float();

        let d_base = magnitude * 5.0;
        let d_steady = Numeric::from(d_base);
        let d_lower = Numeric::from(0.75 * d_base);
        let d_upper = Numeric::from(1.75 * d_base);

        let h_base = magnitude * 4.0 + 270.0;
        let h_steady = Numeric::from(h_base);
        let h_lower = Numeric::from(0.75 * h_base);
        let h_upper = Numeric::from(1.75 * h_base);

        self.effective_diameter.set(d_lower);
        self.effective_diameter.ease_to(d_upper, 30, EasingCurve::Linear);
        self.effective_diameter.ease_to_later(d_steady, 30, EasingCurve::Linear);

        self.effective_hue.set(h_lower);
        self.effective_hue.ease_to(h_upper, 30, EasingCurve::Linear);
        self.effective_hue.ease_to_later(h_steady, 30, EasingCurve::Linear);

    }


    pub fn handle_tick(&mut self, ctx: &NodeContext) {

    }
}
