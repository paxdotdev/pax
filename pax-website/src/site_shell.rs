#![allow(unused_imports)]

use crate::{LogoAnchor, NavLogo, SiteTheme};
use pax_kit::*;

const COMPACT_BREAKPOINT_PX: f64 = 760.0;

#[pax]
#[file("site_shell.pax")]
pub struct SiteShell {
    pub has_hero: Property<bool>,
    pub logo_anchor: Property<LogoAnchor>,
    pub scroll_y: Property<f64>,
    pub hide_nav: Property<bool>,
    pub nav_shown: Property<bool>,
    pub nav_alpha: Property<f64>,
    pub compact: Property<bool>,
    pub safe_top: Property<f64>,
    pub safe_bottom: Property<f64>,
    pub safe_left: Property<f64>,
    pub safe_right: Property<f64>,
}

impl SiteShell {
    pub fn handle_mount(&mut self, ctx: &NodeContext) {
        let bounds = ctx.bounds_self.clone();
        let left = self.safe_left.local();
        let right = self.safe_right.local();
        let deps = [bounds.untyped(), left.untyped(), right.untyped()];
        let compact = self.compact.local();
        let update = move || {
            compact.set_if_neq(bounds.get().0 - left.get() - right.get() < COMPACT_BREAKPOINT_PX);
        };
        update();
        ctx.subscribe(&deps, update);
        let shown = self.should_show_nav();
        self.nav_shown.local().set(shown);
        self.nav_alpha.local().set(if shown { 1.0 } else { 0.0 });
    }

    pub fn handle_pre_render(&mut self, _ctx: &NodeContext) {
        let shown = self.should_show_nav();
        if self.hide_nav.get() {
            self.nav_shown.local().set(false);
            self.nav_alpha.local().cancel_transitions();
            self.nav_alpha.local().set_if_neq(0.0);
        } else if shown != self.nav_shown.get() {
            self.nav_shown.local().set(shown);
            let alpha = self.nav_alpha.local();
            // One playhead fades the complete nav; reversing never queues an old fade.
            alpha.cancel_transitions();
            alpha.ease_to(
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
        let home = self.logo_anchor.get();
        home.ready
            && nav_visible(
                self.nav_shown.get(),
                home.bottom_y - self.scroll_y.get().max(0.0),
                self.safe_top.get() + if self.compact.get() { 64.0 } else { 80.0 },
            )
    }
}

fn nav_visible(shown: bool, hero_bottom: f64, nav_bottom: f64) -> bool {
    let clearance = nav_bottom - hero_bottom;
    // Leave room for the fade-out before the hero returns; hysteresis prevents jitter.
    if shown {
        clearance > 96.0
    } else {
        clearance >= 128.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nav_waits_until_the_entire_hero_logo_is_out() {
        assert!(!nav_visible(false, 200.0, 80.0));
        assert!(!nav_visible(false, 80.0, 80.0));
        assert!(!nav_visible(false, -47.0, 80.0));
        assert!(nav_visible(false, -48.0, 80.0));
    }

    #[test]
    fn reverse_scroll_hides_nav_before_the_hero_returns() {
        assert!(nav_visible(true, -17.0, 80.0));
        assert!(!nav_visible(true, -16.0, 80.0));
        assert!(!nav_visible(true, 80.0, 80.0));
    }

    #[test]
    fn hysteresis_holds_state_and_accounts_for_safe_area() {
        for clearance in 97..128 {
            assert!(nav_visible(true, 108.0 - clearance as f64, 108.0));
            assert!(!nav_visible(false, 108.0 - clearance as f64, 108.0));
        }
    }
}
