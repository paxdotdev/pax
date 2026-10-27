use crate::{DeviceFrame, SiteTheme};
use calculator::Calculator;
use pax_kit::*;

/// A live phone with captured window/tablet wings, or a mobile full-screen link.
/// The desktop calculator mounts and updates with the page, including offscreen.
#[pax]
#[file("calculator_preview.pax")]
pub struct CalculatorPreview {
    pub content_height: Property<f64>,
    pub compact: Property<bool>,
    pub layout_ready: Property<bool>,
    pub stage_scale: Property<f64>,
    pub stage_x: Property<f64>,
    pub devices_height: Property<f64>,
}

impl CalculatorPreview {
    pub fn mount(&mut self, ctx: &NodeContext) {
        let bounds = ctx.bounds_self.clone();
        let compact = self.compact.clone();
        let deps = [bounds.untyped(), compact.untyped()];
        let ready = self.layout_ready.local();
        let scale = self.stage_scale.local();
        let x = self.stage_x.local();
        let height = self.devices_height.local();
        let content_height = self.content_height.local();
        let update = move || {
            // The footer's column breakpoint controls the mode, not the narrower
            // preview column's width. Desktop therefore retains the whole fanout.
            let layout = DeviceLayout::new(bounds.get().0, compact.get());
            scale.set_if_neq(layout.scale * 100.0);
            x.set_if_neq(layout.x);
            height.set_if_neq(layout.height);
            content_height.set_if_neq(layout.height + 76.0);
            ready.set_if_neq(layout.scale > 0.0);
        };
        update();
        ctx.subscribe(&deps, update);
    }
}

struct DeviceLayout {
    scale: f64,
    x: f64,
    height: f64,
}

impl DeviceLayout {
    fn new(width: f64, compact: bool) -> Self {
        let width = if width.is_finite() {
            width.max(0.0)
        } else {
            0.0
        };
        let (design_width, design_height, max_width) = if compact {
            (390.0, 844.0, 312.0)
        } else {
            (880.0, 900.0, 880.0)
        };
        let displayed_width = width.min(max_width);
        let scale = displayed_width / design_width;
        Self {
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
    fn calculator_mount_and_updates_have_no_visibility_gate() {
        let template = include_str!("calculator_preview.pax");
        assert!(!template.contains("if self.loaded"));
        assert!(!template.contains("@viewport_proximity"));
        assert!(template.contains("paused=false"));
    }

    #[test]
    fn invalid_provisional_bounds_do_not_mount_a_preview() {
        for width in [f64::NAN, f64::INFINITY, -1.0, 0.0] {
            for compact in [false, true] {
                let layout = DeviceLayout::new(width, compact);
                assert_eq!(layout.scale, 0.0);
                assert_eq!(layout.height, 0.0);
                assert_eq!(layout.x, 0.0);
            }
        }
    }

    #[test]
    fn both_device_stages_fit_their_column_without_stretching() {
        for width in [160.0, 272.0, 342.0, 390.0, 600.0, 880.0, 1400.0] {
            for compact in [false, true] {
                let layout = DeviceLayout::new(width, compact);
                let (w, h) = if compact {
                    (390.0, 844.0)
                } else {
                    (880.0, 900.0)
                };
                assert!(layout.x >= 0.0);
                assert!(layout.x + w * layout.scale <= width + 0.001);
                assert_eq!(layout.height, h * layout.scale);
                assert!(layout.scale <= 1.0);
            }
        }
    }
}
