use crate::SiteTheme;
use pax_kit::math::Point2;
use pax_kit::*;

// Window-space bottom at scroll zero; only visibility crosses the scroller
// boundary. The hero's actual logo stays in the native scrolling subtree.
#[derive(PartialEq)]
#[pax]
pub struct LogoAnchor {
    pub bottom_y: f64,
    pub ready: bool,
}

#[pax]
#[file("logo_home.pax")]
pub struct LogoHome {
    pub home_bounds: Property<LogoAnchor>,
    pub scroll_y: Property<f64>,
}

impl LogoHome {
    pub fn measure(&mut self, ctx: &NodeContext) {
        let (width, height) = ctx.bounds_self.get();
        if width <= 0.0 || height <= 0.0 {
            return;
        }
        let origin = ctx.local_point(Point2::new(0.0, 0.0));
        let bottom_y = (1.0 - origin.y) * height + self.scroll_y.get();
        if bottom_y.is_finite() {
            self.home_bounds.set_if_neq(LogoAnchor {
                bottom_y,
                ready: true,
            });
        }
    }

    pub fn unmount(&mut self, _: &NodeContext) {
        self.home_bounds.update(|anchor| anchor.ready = false);
    }
}

#[pax]
#[file("nav_logo.pax")]
pub struct NavLogo {}
