#![allow(unused_imports)]

use crate::{NavLogo, SiteTheme};
use pax_kit::*;

const COMPACT_BREAKPOINT_PX: f64 = 760.0;

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
    pub nav_alpha: Property<f64>,
    pub compact: Property<bool>,
    pub safe_top: Property<f64>,
    pub safe_bottom: Property<f64>,
    pub safe_left: Property<f64>,
    pub safe_right: Property<f64>,
    pub hero_present: Property<bool>,
    pub hero_docked: Property<bool>,
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
            // Fade controls and background together; the transferred logo stays visible.
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
        self.hero_present.get() && self.hero_docked.get()
    }

    pub fn on_pre_unmount(&mut self, ctx: &NodeContext) {
        for visitor in ctx.visiting_nodes().all().unwrap_or_default() {
            // The original owner may already be ending with this route.
            let _ = visitor.move_to_home();
        }
    }
}
