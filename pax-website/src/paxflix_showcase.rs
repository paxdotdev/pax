use crate::{DeviceFrame, SiteTheme};
use pax_kit::*;
use paxflix::Paxflix;

/// One live tablet on wide layouts; the other devices are native-app captures.
#[pax]
#[file("paxflix_showcase.pax")]
pub struct PaxflixShowcase {
    pub content_height: Property<f64>,
    pub compact: Property<bool>,
    pub layout_ready: Property<bool>,
    pub stage_scale: Property<f64>,
    pub stage_x: Property<f64>,
    pub devices_height: Property<f64>,
}

impl PaxflixShowcase {
    pub fn mount(&mut self, ctx: &NodeContext) {
        let bounds = ctx.bounds_self.clone();
        let deps = [bounds.untyped()];
        let compact = self.compact.local();
        let ready = self.layout_ready.local();
        let scale = self.stage_scale.local();
        let x = self.stage_x.local();
        let height = self.devices_height.local();
        let content_height = self.content_height.local();
        let update = move || {
            let layout = DeviceLayout::for_width(bounds.get().0);
            compact.set_if_neq(layout.compact);
            scale.set_if_neq(layout.scale * 100.0);
            x.set_if_neq(layout.x);
            height.set_if_neq(layout.height);
            content_height.set_if_neq(layout.height + 76.0);
            // A zero-width provisional layout must never mount the desktop demo.
            ready.set_if_neq(layout.scale > 0.0);
        };
        update();
        ctx.subscribe(&deps, update);
    }
}

struct DeviceLayout {
    compact: bool,
    scale: f64,
    x: f64,
    height: f64,
}

impl DeviceLayout {
    fn for_width(width: f64) -> Self {
        let width = if width.is_finite() {
            width.max(0.0)
        } else {
            0.0
        };
        let compact = width < 760.0;
        let (design_width, design_height, max_width) = if compact {
            (390.0, 844.0, 312.0)
        } else {
            (1280.0, 780.0, 1280.0)
        };
        let displayed_width = width.min(max_width);
        let scale = displayed_width / design_width;
        Self {
            compact,
            scale,
            x: (width - displayed_width) / 2.0,
            height: design_height * scale,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provisional_and_mobile_layouts_never_select_the_live_demo() {
        for width in [f64::NAN, f64::INFINITY, -1.0, 0.0, 272.0, 342.0, 759.0] {
            let layout = DeviceLayout::for_width(width);
            assert!(layout.compact);
            assert!(layout.height.is_finite());
        }
    }

    #[test]
    fn device_stage_fits_and_has_a_stable_aspect_ratio() {
        for width in [272.0, 342.0, 759.0, 760.0, 1126.0, 1800.0] {
            let layout = DeviceLayout::for_width(width);
            let (w, h) = if layout.compact {
                (390.0, 844.0)
            } else {
                (1280.0, 780.0)
            };
            assert!(layout.x >= 0.0);
            assert!(layout.x + w * layout.scale <= width + 0.001);
            assert_eq!(layout.height, h * layout.scale);
        }
    }
}
