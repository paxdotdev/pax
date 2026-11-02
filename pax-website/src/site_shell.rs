#![allow(unused_imports)]

use crate::{NavLogo, SiteTheme};
use pax_kit::math::Point2;
use pax_kit::*;

const COMPACT_BREAKPOINT_PX: f64 = 760.0;
// The emitter is centered at (50% width, 66.5% height). Only viewport geometry
// changes its pose; scroll and pointer state must never move this light.
fn viewport_strip_pose(width: f64, height: f64) -> (f64, f64) {
    let width = if width.is_finite() {
        width.max(0.0)
    } else {
        0.0
    };
    let rise = if height.is_finite() {
        height.max(0.0) * 0.67
    } else {
        0.0
    };
    (width.hypot(rise), (-rise).atan2(width))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_endpoints_match_the_viewport_diagonal_at_each_aspect_ratio() {
        for (width, height) in [
            (390.0, 844.0),
            (1000.0, 652.0),
            (1440.0, 960.0),
            (844.0, 390.0),
        ] {
            let (length, angle) = viewport_strip_pose(width, height);
            let half_x = length * angle.cos() * 0.5;
            let half_y = length * angle.sin() * 0.5;
            let center = (width * 0.5, height * 0.665);
            assert!((center.0 - half_x).abs() < 1e-8);
            assert!((center.1 - half_y - height).abs() < 1e-8);
            assert!((center.0 + half_x - width).abs() < 1e-8);
            assert!((center.1 + half_y - height * 0.33).abs() < 1e-8);
        }
    }

    #[test]
    fn provisional_strip_bounds_remain_finite() {
        for (width, height) in [(0.0, 0.0), (-1.0, 20.0), (f64::NAN, f64::INFINITY)] {
            let (length, angle) = viewport_strip_pose(width, height);
            assert!(length.is_finite() && length >= 0.0);
            assert!(angle.is_finite());
        }
    }

    #[test]
    fn nav_moves_without_group_opacity() {
        let shell = include_str!("site_shell.pax");
        assert!(!shell.contains("opacity="));
        assert!(shell.contains("1 - self.nav_reveal"));
        assert!(shell.contains("self.safe_top + (self.compact ? 64 : 80)"));
    }

    #[test]
    fn strip_is_owned_by_the_shell_not_individual_buttons() {
        let shell = include_str!("site_shell.pax");
        assert_eq!(shell.matches("<StripLight").count(), 1);
        assert!(!include_str!("glass_action.pax").contains("<StripLight"));
        assert!(!include_str!("hero_section.pax").contains("light_progress"));
    }

}

// The shell owns the navigation boundary and whether its route supplies a logo.
// Geometry and presence are shared through the declaration tree, even in flight.
#[derive(Clone)]
pub(crate) struct LogoNavigation {
    pub hero_present: Property<bool>,
    pub hero_docked: Property<bool>,
    pub nav_bottom: LocalProperty<f64>,
    pub hidden: Property<bool>,
}
impl Store for LogoNavigation {}

#[pax]
#[file("site_shell.pax")]
pub struct SiteShell {
    pub has_hero: Property<bool>,
    pub hide_nav: Property<bool>,
    pub nav_shown: Property<bool>,
    pub nav_reveal: Property<f64>,
    pub compact: Property<bool>,
    pub safe_top: Property<f64>,
    pub safe_bottom: Property<f64>,
    pub safe_left: Property<f64>,
    pub safe_right: Property<f64>,
    pub hero_present: Property<bool>,
    pub hero_docked: Property<bool>,
    pub pointer_light_x: Property<f64>,
    pub pointer_light_y: Property<f64>,
    pub pointer_light_active: Property<bool>,
    pub strip_length: Property<f64>,
    pub strip_angle: Property<f64>,
}

impl SiteShell {
    pub fn handle_mount(&mut self, ctx: &NodeContext) {
        let top = self.safe_top.local();
        let compact = self.compact.local();
        let deps = [top.untyped(), compact.untyped()];
        let nav_bottom = LocalProperty::computed(
            move || top.get() + if compact.get() { 64.0 } else { 80.0 },
            &deps,
        );
        ctx.provide_store(LogoNavigation {
            hero_present: self.hero_present.clone(),
            hero_docked: self.hero_docked.clone(),
            nav_bottom,
            hidden: self.hide_nav.clone(),
        })
        .expect("the mounted site shell owns its navigation store");
        let bounds = ctx.bounds_self.clone();
        let left = self.safe_left.local();
        let right = self.safe_right.local();
        let deps = [bounds.untyped(), left.untyped(), right.untyped()];
        let compact = self.compact.local();
        let strip_length = self.strip_length.local();
        let strip_angle = self.strip_angle.local();
        let update = move || {
            let (width, height) = bounds.get();
            compact.set_if_neq(width - left.get() - right.get() < COMPACT_BREAKPOINT_PX);
            let (length, angle) = viewport_strip_pose(width, height);
            strip_length.set_if_neq(length);
            strip_angle.set_if_neq(angle);
        };
        update();
        ctx.subscribe(&deps, update);
        let shown = self.should_show_nav();
        self.nav_shown.local().set(shown);
        self.nav_reveal.local().set(if shown { 1.0 } else { 0.0 });
    }

    pub fn move_pointer_light(&mut self, ctx: &NodeContext, event: Event<MouseMove>) {
        self.place_pointer_light(ctx, event.mouse.x, event.mouse.y);
    }

    pub fn touch_pointer_light(&mut self, ctx: &NodeContext, event: Event<TouchStart>) {
        if let Some(touch) = event.touches.first() {
            self.place_pointer_light(ctx, touch.x, touch.y);
        }
    }

    pub fn drag_pointer_light(&mut self, ctx: &NodeContext, event: Event<TouchMove>) {
        if let Some(touch) = event.touches.first() {
            self.place_pointer_light(ctx, touch.x, touch.y);
        }
    }

    fn place_pointer_light(&mut self, ctx: &NodeContext, x: f64, y: f64) {
        let point = ctx.local_point(Point2::new(x, y));
        let (width, height) = ctx.bounds_self.get();
        // No capture or cancellation: lighting must not interfere with links,
        // controls, or the browser's native touch scrolling.
        self.pointer_light_x.local().set_if_neq(point.x * width);
        self.pointer_light_y.local().set_if_neq(point.y * height);
        self.pointer_light_active.local().set_if_neq(true);
    }

    pub fn handle_pre_render(&mut self, _ctx: &NodeContext) {
        let shown = self.should_show_nav();
        if self.hide_nav.get() {
            self.nav_shown.local().set(false);
            self.nav_reveal.local().cancel_transitions();
            self.nav_reveal.local().set_if_neq(0.0);
        } else if shown != self.nav_shown.get() {
            self.nav_shown.local().set(shown);
            let reveal = self.nav_reveal.local();
            // Move the opaque nav as one surface; reversal starts from its current pose.
            reveal.cancel_transitions();
            reveal.ease_to(
                if shown { 1.0 } else { 0.0 },
                Duration::Milliseconds(if shown { 500.into() } else { 450.into() }),
                EasingCurve::InOutQuad,
            );
        }
    }

    fn should_show_nav(&self) -> bool {
        if self.hide_nav.get() {
            return false;
        }
        if !self.has_hero.get() {
            return true;
        }
        self.hero_present.get() && self.hero_docked.get()
    }

    pub fn on_pre_unmount(&mut self, ctx: &NodeContext) {
        for visitor in ctx.visiting_nodes().all().unwrap_or_default() {
            // The original owner may already be ending with this route.
            let _ = visitor.move_to_home();
        }
    }
}
