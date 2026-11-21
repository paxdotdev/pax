use crate::SiteTheme;
use pax_kit::*;

// Both logo instances use the same 12-second sweep in artwork coordinates,
// but neither depends on the other's visibility-gated animation clock.
pub(crate) fn idle_light_position(elapsed_ms: f64) -> (f64, f64) {
    let phase = (elapsed_ms.max(0.0) / 12000.0).rem_euclid(1.0);
    let cosine = (phase * std::f64::consts::TAU).cos();
    (490.0 - 462.0 * cosine, 205.0 + 28.0 * cosine)
}

#[pax]
#[file("site_logo.pax")]
pub struct SiteLogo {
    pub light_mode: Property<bool>,
    pub radiance_enabled: Property<bool>,
    pub light_x: Property<f64>,
    pub light_y: Property<f64>,
    pub brightness: Property<f64>,
    pub progress: Property<f64>,
    pub click_to_replay: Property<bool>,
    pub logo_scale: Property<f64>,
}

impl SiteLogo {
    pub fn toggle_theme(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.light_mode.set(!self.light_mode.get());
    }

    pub fn hover(&mut self, ctx: &NodeContext, _: Event<MouseOver>) {
        ctx.set_cursor(cursor::CursorStyle::Pointer);
    }

    pub fn leave(&mut self, ctx: &NodeContext, _: Event<MouseOut>) {
        ctx.set_cursor(cursor::CursorStyle::Auto);
    }

    pub fn on_mount(&mut self, ctx: &NodeContext) {
        let bounds = ctx.bounds_self.clone();
        let deps = [bounds.untyped()];
        self.logo_scale.replace_with(LocalProperty::computed(
            move || {
                let (width, height) = bounds.get();
                (width / 920.72).min(height / 463.95).max(0.0) * 100.0
            },
            &deps,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idle_light_pans_both_ways_and_loops_at_the_intro_handoff() {
        for (time, expected) in [
            (0.0, (28.0, 233.0)),
            (3000.0, (490.0, 205.0)),
            (6000.0, (952.0, 177.0)),
            (9000.0, (490.0, 205.0)),
            (12000.0, (28.0, 233.0)),
        ] {
            let (x, y) = idle_light_position(time);
            assert!((x - expected.0).abs() < 1e-8);
            assert!((y - expected.1).abs() < 1e-8);
        }
        assert_eq!(idle_light_position(-100.0), idle_light_position(0.0));
        let first = idle_light_position(1234.0);
        let later = idle_light_position(121234.0);
        assert!((first.0 - later.0).abs() < 1e-8);
        assert!((first.1 - later.1).abs() < 1e-8);
    }
}
