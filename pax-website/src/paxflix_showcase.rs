use crate::DeviceFrame;
use crate::SiteTheme;
use pax_kit::*;
use paxflix::Paxflix;

/// Two independent live apps, sharing canonical source and artwork.
#[pax]
#[file("paxflix_showcase.pax")]
pub struct PaxflixShowcase {
    pub content_height: Property<f64>,
    pub tablet_scale: Property<f64>,
    pub phone_scale: Property<f64>,
    pub tablet_x: Property<f64>,
    pub tablet_y: Property<f64>,
    pub phone_x: Property<f64>,
    pub phone_y: Property<f64>,
    pub loaded: Property<bool>,
    pub demo_open: Property<bool>,
    pub devices_height: Property<f64>,
}

impl PaxflixShowcase {
    pub fn layout(&mut self, ctx: &NodeContext) {
        let layout = DeviceLayout::for_width(ctx.bounds_self.get().0);
        self.devices_height.set_if_neq(layout.height);
        self.content_height.set_if_neq(layout.height + 64.0);
        self.tablet_scale.set_if_neq(layout.tablet_scale * 100.0);
        self.phone_scale.set_if_neq(layout.phone_scale * 100.0);
        self.tablet_x.set_if_neq(layout.tablet_x);
        self.tablet_y.set_if_neq(layout.tablet_y);
        self.phone_x.set_if_neq(layout.phone_x);
        self.phone_y.set_if_neq(layout.phone_y);
    }

    pub fn prepare(&mut self, _: &NodeContext, _: Event<ViewportProximityEnter>) {
        // Retain the two sessions after their first visit. Paxflix's shelves
        // defer their card trees, and cards reveal their copy on viewport entry.
        self.loaded.set(true);
    }

    pub fn expand(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.demo_open.set(true);
    }
}

struct DeviceLayout {
    tablet_scale: f64,
    phone_scale: f64,
    tablet_x: f64,
    tablet_y: f64,
    phone_x: f64,
    phone_y: f64,
    height: f64,
}

impl DeviceLayout {
    fn for_width(width: f64) -> Self {
        let width = width.max(0.0);
        let side_by_side = width >= 820.0;
        let phone_width = if side_by_side {
            (width * 0.29).clamp(280.0, 356.0)
        } else {
            width.min(356.0)
        };
        let tablet_width = if side_by_side {
            (width - phone_width - 32.0).min(1048.0)
        } else {
            width.min(1048.0)
        };
        let tablet_height = tablet_width * 812.0 / 1048.0;
        let phone_height = phone_width * 844.0 / 390.0;
        let height = if side_by_side {
            tablet_height.max(phone_height)
        } else {
            tablet_height + 32.0 + phone_height
        };
        let left = (width - tablet_width - 32.0 - phone_width) / 2.0;
        Self {
            tablet_scale: tablet_width / 1048.0,
            phone_scale: phone_width / 390.0,
            tablet_x: if side_by_side {
                left
            } else {
                (width - tablet_width) / 2.0
            },
            tablet_y: if side_by_side {
                (height - tablet_height) / 2.0
            } else {
                0.0
            },
            phone_x: if side_by_side {
                left + tablet_width + 32.0
            } else {
                (width - phone_width) / 2.0
            },
            phone_y: if side_by_side {
                (height - phone_height) / 2.0
            } else {
                tablet_height + 32.0
            },
            height,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn devices_fit_without_overlap_at_mobile_and_desktop_widths() {
        for width in [272.0, 342.0, 528.0, 819.0, 820.0, 1126.0, 1800.0] {
            let l = DeviceLayout::for_width(width);
            let tw = 1048.0 * l.tablet_scale;
            let th = 812.0 * l.tablet_scale;
            let pw = 390.0 * l.phone_scale;
            let ph = 844.0 * l.phone_scale;
            assert!(l.tablet_x >= 0.0 && l.phone_x >= 0.0);
            assert!(l.tablet_x + tw <= width + 0.001);
            assert!(l.phone_x + pw <= width + 0.001);
            assert!(l.tablet_y + th <= l.height + 0.001);
            assert!(l.phone_y + ph <= l.height + 0.001);
            assert!(l.phone_x >= l.tablet_x + tw + 31.99 || l.phone_y >= th + 31.99);
            assert!(pw >= width.min(280.0));
        }
    }
}
