use pax_engine::*;
use pax_engine::api::*;

use pax_std::primitives::Ellipse;
use rand::random;

#[pax]
#[file("ball.pax")]
pub struct Ball {
    /// Expected [0,1]
    pub magnitude: Property<Numeric>,
    pub effective_diameter: Property<Numeric>,
    pub effective_hue: Property<Numeric>,
    pub index: Property<Numeric>,
}

impl Ball {
    pub fn handle_mount(&mut self, ctx: &NodeContext) {
        let magnitude = self.magnitude.get().to_float();

        let d_base = 45.0 + (magnitude * 3.0);
        let d_steady = Numeric::from(d_base);
        let d_lower = Numeric::from(0.85 * d_base);
        let d_upper = Numeric::from(1.05 * d_base);

        let h_base = magnitude * 2.0 + 270.0;
        let h_steady = Numeric::from(h_base);
        let h_lower = Numeric::from(0.85 * h_base);
        let h_upper = Numeric::from(1.05 * h_base);

        let delay : u64 = (3.0 + (random::<f64>()) * self.index.get().to_float()) as u64;

        self.effective_diameter.set(0.into());
        self.effective_diameter.ease_to(0.into(), delay, EasingCurve::Linear);
        self.effective_diameter.ease_to_later(d_lower, 20, EasingCurve::OutQuad);
        self.effective_diameter.ease_to_later(d_upper, 30, EasingCurve::OutQuad);
        self.effective_diameter.ease_to_later(d_steady, 30, EasingCurve::InQuad);

        self.effective_hue.set(h_lower);
        self.effective_hue.ease_to(h_lower, 20, EasingCurve::OutQuad);
        self.effective_hue.ease_to_later(h_upper, 30, EasingCurve::OutQuad);
        self.effective_hue.ease_to_later(h_steady, 30, EasingCurve::InQuad);

    }


    pub fn handle_tick(&mut self, ctx: &NodeContext) {

    }
}
