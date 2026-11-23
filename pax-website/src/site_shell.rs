#![allow(unused_imports)]

use crate::{SiteHeader, SiteLogo, SiteTheme};
use pax_kit::*;

const COMPACT_BREAKPOINT_PX: f64 = 760.0;

#[cfg(test)]
mod tests {
    #[test]
    fn nav_moves_without_group_opacity() {
        let shell = include_str!("site_shell.pax");
        assert!(!shell.contains("opacity="));
        assert!(shell.contains("(self.nav_reveal - 1) * self.island_height"));
        assert!(shell.contains("self.safe_top + self.island_gap"));
        assert!(shell.contains("height={(self.safe_top)px}"));
    }

    #[test]
    fn nav_light_ticks_only_for_the_visible_gpu_logo() {
        let shell = include_str!("site_shell.pax");
        assert!(shell
            .contains("if self.has_hero && !self.hide_nav && self.nav_logo_entry > 0.0 && @gpu"));
        assert!(shell.contains("@tick=self.advance_nav_light"));
        assert!(shell.contains("light_x={self.nav_light_x} light_y={self.nav_light_y}"));
        assert!(!shell.contains("light_x=28 light_y=233"));
    }

    #[test]
    fn site_keeps_pointer_lighting_without_a_strip() {
        let routes = include_str!("lib.pax");
        assert!(!routes.contains("<StripLight"));
        assert!(routes.contains("<LightSource class=\"pointer_light\""));
        assert!(routes.contains("<AmbientLight intensity=1"));
        assert!(!include_str!("glass_action.pax").contains("<StripLight"));
    }
}

// The shell owns the navigation boundary and whether its route supplies a logo.
// The two logos share a clock, not presentation ownership or shader captures.
#[derive(Clone)]
pub(crate) struct LogoNavigation {
    pub hero_present: Property<bool>,
    pub hero_docked: Property<bool>,
    pub nav_bottom: LocalProperty<f64>,
    pub hidden: Property<bool>,
    pub portal_progress: Property<f64>,
    pub nav_logo_width: LocalProperty<f64>,
}
impl Store for LogoNavigation {}

#[pax]
#[file("site_shell.pax")]
pub struct SiteShell {
    pub light_mode: Property<bool>,
    pub has_hero: Property<bool>,
    pub hide_nav: Property<bool>,
    pub nav_shown: Property<bool>,
    pub nav_reveal: Property<f64>,
    pub portal_progress: Property<f64>,
    pub nav_logo_entry: Property<f64>,
    pub nav_light_x: Property<f64>,
    pub nav_light_y: Property<f64>,
    pub compact: Property<bool>,
    pub island_width: Property<f64>,
    pub island_height: Property<f64>,
    pub island_gap: Property<f64>,
    pub safe_top: Property<f64>,
    pub safe_bottom: Property<f64>,
    pub safe_left: Property<f64>,
    pub safe_right: Property<f64>,
    pub hero_present: Property<bool>,
    pub hero_docked: Property<bool>,
}

impl SiteShell {
    pub fn handle_mount(&mut self, ctx: &NodeContext) {
        self.advance_nav_light(ctx);
        let top = self.safe_top.local();
        let height = self.island_height.local();
        let gap = self.island_gap.local();
        let deps = [top.untyped(), height.untyped(), gap.untyped()];
        let nav_bottom = LocalProperty::computed(
            move || top.get() + gap.get() + height.get(),
            &deps,
        );
        let compact = self.compact.local();
        let nav_logo_width = LocalProperty::computed(
            move || SiteHeader::logo_width(compact.get()),
            &[self.compact.untyped()],
        );
        ctx.provide_store(LogoNavigation {
            hero_present: self.hero_present.clone(),
            hero_docked: self.hero_docked.clone(),
            nav_bottom,
            hidden: self.hide_nav.clone(),
            portal_progress: self.portal_progress.clone(),
            nav_logo_width,
        })
        .expect("the mounted site shell owns its navigation store");
        let bounds = ctx.bounds_self.clone();
        let left = self.safe_left.local();
        let right = self.safe_right.local();
        let deps = [bounds.untyped(), left.untyped(), right.untyped()];
        let compact = self.compact.local();
        let island_width = self.island_width.local();
        let island_height = self.island_height.local();
        let island_gap = self.island_gap.local();
        let update = move || {
            let (width, _) = bounds.get();
            let available = width - left.get() - right.get();
            let small = available < COMPACT_BREAKPOINT_PX;
            compact.set_if_neq(small);
            island_width.set_if_neq(if small { (available - 20.0).clamp(0.0, 312.0) } else { 552.0 });
            island_height.set_if_neq(if small { 48.0 } else { 52.0 });
            island_gap.set_if_neq(if small { 8.0 } else { 14.0 });
        };
        update();
        ctx.subscribe(&deps, update);
        let shown = self.should_show_nav();
        self.nav_shown.local().set(shown);
        self.nav_reveal.local().set(if shown { 1.0 } else { 0.0 });
        let progress = self.portal_progress.local();
        let has_hero = self.has_hero.local();
        let deps = [progress.untyped(), has_hero.untyped()];
        self.nav_reveal.replace_with(LocalProperty::computed(
            move || {
                if has_hero.get() {
                    crate::logo_portal::nav_reveal(progress.get())
                } else {
                    progress.get()
                }
            },
            &deps,
        ));
        let progress = self.portal_progress.local();
        self.nav_logo_entry.replace_with(LocalProperty::computed(
            move || crate::logo_portal::nav_entry(progress.get()),
            &[self.portal_progress.untyped()],
        ));
        self.portal_progress
            .local()
            .set(if shown { 1.0 } else { 0.0 });
    }

    pub fn advance_nav_light(&mut self, ctx: &NodeContext) {
        // The hero clock pauses offscreen. Sample the runtime clock here instead;
        // a visibility-gated tick drives only the nav twin's shader uniforms.
        let (x, y) = crate::site_logo::idle_light_position(ctx.elapsed_time_millis() as f64);
        self.nav_light_x.local().set_if_neq(x);
        self.nav_light_y.local().set_if_neq(y);
    }

    pub fn handle_pre_render(&mut self, _ctx: &NodeContext) {
        let shown = self.should_show_nav();
        if self.hide_nav.get() {
            self.nav_shown.local().set(false);
            self.portal_progress.local().cancel_transitions();
            self.portal_progress.local().set_if_neq(0.0);
        } else if shown != self.nav_shown.get() {
            self.nav_shown.local().set(shown);
            let reveal = self.portal_progress.local();
            // Reversal samples the same choreography from its current pose.
            let target = if shown { 1.0 } else { 0.0 };
            let duration =
                (crate::logo_portal::DURATION_MS * (target - reveal.get()).abs()).max(1.0);
            reveal.cancel_transitions();
            reveal.ease_to(
                target,
                Duration::Milliseconds(duration.into()),
                EasingCurve::Linear,
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

}
