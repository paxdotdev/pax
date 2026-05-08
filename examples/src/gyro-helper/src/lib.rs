#![allow(unused_imports)]

use pax_kit::*;

#[pax]
#[main]
#[file("lib.pax")]
pub struct Example {
    pub gyro_label: Property<String>,
    pub accel_label: Property<String>,
    pub status_label: Property<String>,
    pub wobble: Property<f64>,
    pub accel_x: Property<f64>,
    pub accel_y: Property<f64>,
    pub accel_energy: Property<f64>,
    pub glow: Property<f64>,
}

impl Example {
    pub fn on_mount(&mut self, _ctx: &NodeContext) {
        self.gyro_label
            .set("GYRO x +00.0 y +00.0 z +00.0".to_string());
        self.accel_label
            .set("ACCEL x +00.0 y +00.0 z +00.0".to_string());
        self.status_label
            .set("Tilt a mobile device, or use a browser sensor emulator.".to_string());
        self.accel_x.set(0.0);
        self.accel_y.set(0.0);
        self.glow.set(0.74);
    }

    pub fn on_gyro(&mut self, _ctx: &NodeContext, event: Event<Gyro>) {
        self.gyro_label.set(format!(
            "GYRO x {:+05.1} y {:+05.1} z {:+05.1}",
            event.x, event.y, event.z
        ));
        let target = (event.y * 0.72 + event.x * 0.24).clamp(-34.0, 34.0);
        self.wobble.ease_to(target, 10, EasingCurve::OutQuad);
        self.status_label.set("Live @gyro handler".to_string());
    }

    pub fn on_accel(&mut self, ctx: &NodeContext, event: Event<Accel>) {
        self.accel_label.set(format!(
            "ACCEL x {:+05.1} y {:+05.1} z {:+05.1}",
            event.x, event.y, event.z
        ));
        let energy = (event.x.abs() + event.y.abs() + event.z.abs()).clamp(0.0, 28.0);
        self.accel_x
            .ease_to(event.x.clamp(-18.0, 18.0), 16, EasingCurve::OutQuad);
        self.accel_y
            .ease_to(event.y.clamp(-18.0, 18.0), 16, EasingCurve::OutQuad);
        self.accel_energy.ease_to(energy, 12, EasingCurve::OutQuad);
        let gyro = ctx.gyro.get();
        let glow = (0.42 + energy / 36.0 + gyro.y.abs() / 180.0).clamp(0.42, 1.0);
        self.glow.ease_to(glow, 12, EasingCurve::OutQuad);
        self.status_label.set("Live @accel handler".to_string());
    }
}
