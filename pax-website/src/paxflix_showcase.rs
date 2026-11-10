use crate::{DeviceFrame, SiteTheme};
use pax_kit::*;
use paxflix::Paxflix;

/// A full-width device fan: one live desktop tablet, linked captures on mobile.
#[pax]
#[file("paxflix_showcase.pax")]
pub struct PaxflixShowcase {
    pub content_height: Property<f64>,
    pub compact: Property<bool>,
    pub layout_ready: Property<bool>,
    pub stage_scale: Property<f64>,
    pub devices_height: Property<f64>,
    pub scene_progress: Property<f64>,
}

impl PaxflixShowcase {
    pub fn mount(&mut self, ctx: &NodeContext) {
        let bounds = ctx.bounds_self.clone();
        let deps = [bounds.untyped()];
        let ready = self.layout_ready.local();
        let scale = self.stage_scale.local();
        let height = self.devices_height.local();
        let content_height = self.content_height.local();
        let update = move || {
            // Only the outer stage scales. The live app always lays out at the
            // tablet's logical resolution, even while its frame is choreographed.
            let layout = DeviceLayout::for_width(bounds.get().0);
            scale.set_if_neq(layout.scale * 100.0);
            height.set_if_neq(layout.height);
            content_height.set_if_neq(layout.height + 76.0);
            // Do not render a provisional zero-sized stage.
            ready.set_if_neq(layout.scale > 0.0);
        };
        update();
        ctx.subscribe(&deps, update);
    }
}

struct DeviceLayout {
    scale: f64,
    height: f64,
}

impl DeviceLayout {
    fn for_width(width: f64) -> Self {
        let width = if width.is_finite() {
            width.max(0.0)
        } else {
            0.0
        };
        let scale = width / 1280.0;
        Self {
            scale,
            height: 780.0 * scale,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provisional_bounds_do_not_mount_a_live_app() {
        for width in [f64::NAN, f64::INFINITY, -1.0, 0.0] {
            let layout = DeviceLayout::for_width(width);
            assert_eq!(layout.scale, 0.0);
            assert_eq!(layout.height, 0.0);
        }
    }

    #[test]
    fn fan_fills_the_available_width_without_a_desktop_or_mobile_cap() {
        for width in [272.0, 342.0, 540.0, 760.0, 1126.0, 1800.0, 2560.0] {
            let layout = DeviceLayout::for_width(width);
            assert!((1280.0 * layout.scale - width).abs() < 0.001);
            assert_eq!(layout.height, 780.0 * layout.scale);
        }
    }

    #[test]
    fn only_the_desktop_center_is_live_and_all_layouts_keep_three_devices() {
        let template = include_str!("paxflix_showcase.pax");
        assert_eq!(template.matches("<Paxflix ").count(), 1);
        assert!(
            template.contains("embedded=true disable_keyboard=true disable_card_entrances=true")
        );
        assert_eq!(template.matches("url=\"/paxflix/\"").count(), 4);
        assert_eq!(template.matches("id=front_tablet").count(), 1);
        assert_eq!(template.matches("id=phone_wing").count(), 1);
        assert_eq!(template.matches("id=desktop_wing").count(), 1);
        assert!(template.contains("if self.compact"));
        assert!(template.contains("#desktop_wing"));
        assert!(template.contains("#phone_wing"));
        assert!(template.contains("#front_tablet"));
    }
}
