use crate::SiteTheme;
use pax_kit::*;
use rain_pane::WetPane;

#[pax]
#[file("fluent_rain.pax")]
pub struct FluentRain {
    pub light_mode: Property<bool>,
    pub visible: Property<bool>,
    pub light_time: Property<f64>,
    pub cyan_x: Property<f64>,
    pub cyan_y: Property<f64>,
    pub magenta_x: Property<f64>,
    pub magenta_y: Property<f64>,
    pub yellow_x: Property<f64>,
    pub yellow_y: Property<f64>,
    pub light_radius: Property<f64>,
    pub light_depth: Property<f64>,
    #[pax(rust)]
    pub last_ms: f64,
}

impl FluentRain {
    pub fn mount(&mut self, ctx: &NodeContext) {
        self.last_ms = ctx.elapsed_time_millis() as f64;
        for (target, light, axis) in [
            (&self.cyan_x, 0, 0),
            (&self.cyan_y, 0, 1),
            (&self.magenta_x, 1, 0),
            (&self.magenta_y, 1, 1),
            (&self.yellow_x, 2, 0),
            (&self.yellow_y, 2, 1),
        ] {
            let time = self.light_time.local();
            let bounds = ctx.bounds_self.clone();
            let deps = [time.untyped(), bounds.untyped()];
            target.replace_with(LocalProperty::computed(
                move || searchlight_pose(time.get(), light, bounds.get())[axis],
                &deps,
            ));
        }
        for (target, depth) in [(&self.light_radius, false), (&self.light_depth, true)] {
            let bounds = ctx.bounds_self.clone();
            let deps = [bounds.untyped()];
            target.replace_with(LocalProperty::computed(
                move || {
                    let extent = bounds.get().0.max(bounds.get().1);
                    if depth {
                        // Height broadens the specular footprint; radius alone
                        // only changes attenuation and cannot fill the dark gaps.
                        (extent * 0.40).clamp(480.0, 1000.0)
                    } else {
                        (extent * 1.8).max(1800.0)
                    }
                },
                &deps,
            ));
        }
    }

    pub fn visibility(&mut self, ctx: &NodeContext, event: Event<ViewportProximityChange>) {
        self.visible.set_if_neq(event.current.is_in_viewport());
        self.last_ms = ctx.elapsed_time_millis() as f64;
    }

    pub fn animate(&mut self, ctx: &NodeContext) {
        let now = ctx.elapsed_time_millis() as f64;
        let elapsed = now - self.last_ms;
        self.last_ms = now;
        // One clock for the rain field and its scoped lights.
        // Do not catch up a hidden tab or animate an offscreen panel.
        if self.visible.get() && (0.0..100.0).contains(&elapsed) {
            self.light_time
                .set(self.light_time.get() + elapsed / 1000.0);
        }
    }
}

// Staggered, smooth search sweeps: each light visits the wet panel without
// all three repeatedly converging at the same point or reversing together.
fn searchlight_pose(seconds: f64, light: usize, bounds: (f64, f64)) -> [f64; 2] {
    let phase = light as f64 * std::f64::consts::TAU / 3.0;
    let angle = seconds * std::f64::consts::TAU / [37.0, 43.0, 49.0][light];
    [
        bounds.0 * (0.5 + 0.40 * (angle + phase).cos()),
        bounds.1 * (0.5 + 0.36 * (angle * 1.37 + phase).sin()),
    ]
}

#[cfg(test)]
mod lighting_tests {
    use super::*;

    #[test]
    fn searchlights_cover_the_copy_panel() {
        for bounds in [(390.0, 610.0), (1440.0, 410.0), (2400.0, 345.0)] {
            for light in 0..3 {
                let mut low: f64 = 1.0;
                let mut high: f64 = 0.0;
                for frame in 0..6000 {
                    let [x, y] = searchlight_pose(frame as f64 / 60.0, light, bounds);
                    assert!((0.09 * bounds.0..=0.91 * bounds.0).contains(&x));
                    assert!((0.13 * bounds.1..=0.87 * bounds.1).contains(&y));
                    low = low.min(y / bounds.1);
                    high = high.max(y / bounds.1);
                }
                assert!(low < 0.15 && high > 0.85);
            }
        }
    }

    #[test]
    fn searchlight_motion_is_continuous_and_resizes_proportionally() {
        for light in 0..3 {
            for frame in 0..6000 {
                let t = frame as f64 / 60.0;
                let a = searchlight_pose(t, light, (1440.0, 1200.0));
                let b = searchlight_pose(t + 1.0 / 60.0, light, (1440.0, 1200.0));
                let doubled = searchlight_pose(t, light, (2880.0, 2400.0));
                for axis in 0..2 {
                    assert!((b[axis] - a[axis]).abs() < 2.0);
                    assert_eq!(doubled[axis], a[axis] * 2.0);
                }
            }
        }
    }
}
