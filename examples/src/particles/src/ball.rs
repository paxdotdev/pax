use pax_engine::api::*;
use pax_engine::*;

use pax_std::*;
use rand::random;

#[pax]
#[file("ball.pax")]
pub struct Ball {
    /// Expected [0,1]
    pub magnitude: Property<Numeric>,
    pub diameter: Property<Numeric>,
    pub hue: Property<Numeric>,
    pub index: Property<Numeric>,
}

impl Ball {
    pub fn handle_mount(&mut self, ctx: &NodeContext) {
        self.update(ctx);
    }

    pub fn update(&mut self, ctx: &NodeContext) {
        if ctx.frames_elapsed.get() % (crate::LOOP_FRAMES as u64) == 0 {
            let magnitude = self.magnitude.get().to_float();
            self.magnitude.set(random::<f64>().into());

            let d_base = 155.0 + (magnitude * 75.5);
            let d_steady = Numeric::from(d_base);
            let d_lower = Numeric::from(0.85 * d_base);
            let d_upper = Numeric::from(1.15 * d_base);

            let h_base = magnitude * 55.0 + 270.0;
            let h_steady = Numeric::from(h_base);
            let h_lower = Numeric::from(0.85 * h_base);
            let h_upper = Numeric::from(1.25 * h_base);

            let seq_progress_0_1 = self.index.get().to_float() / (crate::PARTICLE_COUNT as f64);

            let delay_frames: u64 =
                ((1.0 - seq_progress_0_1) * (random::<f64>() * crate::LOOP_FRAMES)) as u64;

            self.diameter
                .ease_to(0.into(), delay_frames, EasingCurve::Linear);
            self.diameter
                .ease_to_later(d_lower, 20, EasingCurve::OutQuad);
            self.diameter
                .ease_to_later(d_upper, 40, EasingCurve::OutQuad);
            self.diameter
                .ease_to_later(d_steady, 40, EasingCurve::InQuad);

            self.hue
                .ease_to(h_lower, delay_frames, EasingCurve::OutQuad);
            self.hue.ease_to_later(h_upper, 40, EasingCurve::OutQuad);
            self.hue.ease_to_later(h_steady, 40, EasingCurve::InQuad);
        }
    }

    pub fn handle_tick(&mut self, ctx: &NodeContext) {
        self.update(ctx);
    }
}
