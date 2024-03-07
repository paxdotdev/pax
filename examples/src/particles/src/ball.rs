use pax_engine::*;
use pax_engine::api::*;

use pax_std::primitives::Ellipse;
use rand::random;

#[pax]
#[file("ball.pax")]
pub struct Ball {
    pub magnitude: Property<Numeric>,
    pub effective_diameter: Property<Numeric>,
    pub effective_hue: Property<Numeric>,
    pub index: Property<Numeric>,
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
        let h_lower = Numeric::from(0.96 * h_base);
        let h_upper = Numeric::from(1.04 * h_base);

        let delay : u64 = (3.0 + (random::<f64>()) * self.index.get().to_float()) as u64;

        self.effective_diameter.set(0.into());
        self.effective_diameter.ease_to(0.into(), delay, EasingCurve::Linear);
        self.effective_diameter.ease_to_later(d_lower, 20, EasingCurve::OutQuad);
        self.effective_diameter.ease_to_later(d_upper, 50, EasingCurve::OutQuad);
        self.effective_diameter.ease_to_later(d_steady, 40, EasingCurve::InQuad);

        self.effective_hue.set(0.into());
        self.effective_hue.ease_to(0.into(), delay, EasingCurve::Linear);
        self.effective_hue.ease_to_later(h_lower, 20, EasingCurve::OutQuad);
        self.effective_hue.ease_to(h_upper, 50, EasingCurve::OutQuad);
        self.effective_hue.ease_to_later(h_steady, 40, EasingCurve::InQuad);

    }


    pub fn handle_tick(&mut self, ctx: &NodeContext) {

    }
}
