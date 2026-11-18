use crate::{site_shell::LogoNavigation, GetStarted, GlassAction, RotatingHeadline, SiteLogo, SiteTheme};
use pax_kit::*;

#[pax]
#[file("hero_section.pax")]
pub struct HeroSection {
    pub get_started_height: Property<f64>,
    pub content_height: Property<f64>,
    pub _height_bound: Property<bool>,
    pub compact: Property<bool>,
    pub font_px: Property<f64>,
    pub logo_width: Property<f64>,
    pub logo_progress: Property<f64>,
    pub radiance_enabled: Property<bool>,
    pub logo_visible: Property<bool>,
    pub playhead_ms: Property<f64>,
    pub intro_light_x: Property<f64>,
    pub intro_light_y: Property<f64>,
    pub light_brightness: Property<f64>,
    pub idle_light_x: Property<f64>,
    pub idle_light_y: Property<f64>,
    pub _light_last_ms: Property<f64>,
    pub _vertical_padding: Property<f64>,
    pub logo_docked: Property<bool>,
    pub _portal_ready: Property<bool>,
    pub portal_departure: Property<f64>,
    pub portal_lift: Property<f64>,
    pub portal_scale: Property<f64>,
    pub _last_ms: Property<f64>,
    pub _last_content_height: Property<f64>,
    pub _startup_quiet_ms: Property<f64>,
    pub _startup_deadline_ms: Property<f64>,
    pub _logo_started: Property<bool>,
    pub _logo_was_docked: Property<bool>,
    pub _logo_return_pending: Property<bool>,
    pub scroll_y: Property<f64>,
    pub exit_progress: Property<f64>,
}

impl HeroSection {
    pub fn handle_mount(&mut self, ctx: &NodeContext) {
        let now = ctx.elapsed_time_millis() as f64;
        self._last_ms.set(now);
        self._light_last_ms.set(now);
        self.logo_visible.set(true);
        self._last_content_height.set(self.content_height.get());
        self._startup_deadline_ms.set(now + 5000.0);
        self._height_bound.set(false);
        self.bind_layout(ctx);
        if let Ok(present) = ctx.with_store(|nav: &mut LogoNavigation| nav.hero_present.clone()) {
            present.local().set(true);
        }
        if let Ok(nav) = ctx.with_store(|nav: &mut LogoNavigation| nav.clone()) {
            let progress = nav.portal_progress.local();
            self.portal_departure.replace_with(LocalProperty::computed(
                move || crate::logo_portal::hero_departure(progress.get()),
                &[nav.portal_progress.untyped()],
            ));
            let width = self.logo_width.local();
            let nav_width = nav.nav_logo_width.clone();
            let deps = [width.untyped(), nav_width.untyped()];
            self.portal_scale.replace_with(LocalProperty::computed(
                move || (nav_width.get() * 1.35 / width.get().max(1.0)).min(1.0),
                &deps,
            ));
            let scroll_y = self.scroll_y.clone();
            ctx.subscribe(&[nav.home_request.untyped()], move || scroll_y.set(0.0));
        }
    }

    pub fn handle_pre_render(&mut self, ctx: &NodeContext) {
        // The home mount handler runs before all of its descendants and the nav
        // exist. Arm its subscription once the initial tree has been mounted.
        self._portal_ready.set_if_neq(true);
        let previous_height = self._last_content_height.get();
        if !self._height_bound.get() {
            self._height_bound
                .set(crate::section_layout::bind_height_with_padding(
                    ctx,
                    "hero_content",
                    &self.content_height,
                    &self._vertical_padding,
                ));
        }
        let height = self.content_height.get();
        self._last_content_height.set(height);
        let now = ctx.elapsed_time_millis() as f64;
        let elapsed = (now - self._last_ms.get()).max(0.0);
        self._last_ms.set(now);
        self.update_logo_return(ctx, now);
        if !self._logo_started.get() {
            // Start on a settled run of frames, after expensive tree/layout work,
            // instead of advancing the reveal in steps during blocked frames.
            let quiet = if elapsed <= 40.0 && (previous_height - height).abs() < 0.5 {
                self._startup_quiet_ms.get() + elapsed
            } else {
                0.0
            };
            self._startup_quiet_ms.set(quiet);
            // A persistently low frame rate must not suppress the logo forever.
            if quiet < 100.0 && now < self._startup_deadline_ms.get() {
                return;
            }
            self._logo_started.set(true);
            self._light_last_ms.set(now);
        }
    }

    pub fn advance_logo(&mut self, ctx: &NodeContext) {
        let now = ctx.elapsed_time_millis() as f64;
        let elapsed = (now - self._light_last_ms.get()).max(0.0);
        self._light_last_ms.set(now);
        let millis = self.playhead_ms.get() + elapsed;
        self.playhead_ms.set(millis);
        // The idle sweep starts at the intro's stationary handoff at 2100ms.
        let (x, y) = crate::site_logo::idle_light_position(millis - 2100.0);
        self.idle_light_x.set(x);
        self.idle_light_y.set(y);
    }

    pub fn logo_visibility(&mut self, ctx: &NodeContext, event: Event<ViewportProximityChange>) {
        self.logo_visible.set(event.current.is_in_viewport());
        self._light_last_ms.set(ctx.elapsed_time_millis() as f64);
    }

    pub fn replay_logo(&mut self, ctx: &NodeContext, _: Event<Click>) {
        self._logo_return_pending.set(false);
        self.playhead_ms.set(0.0);
        self._logo_started.set(true);
        self._light_last_ms.set(ctx.elapsed_time_millis() as f64);
    }

    fn update_logo_return(&mut self, ctx: &NodeContext, now: f64) {
        let docked = self.logo_docked.get();
        let was_docked = self._logo_was_docked.get();
        let pending = self._logo_return_pending.get();
        if docked == was_docked && !pending {
            return;
        }
        let hidden = ctx
            .with_store(|nav: &mut LogoNavigation| nav.hidden.get())
            .unwrap_or(false);
        let action = logo_return_action(
            was_docked,
            docked,
            hidden,
            pending,
            self.portal_departure.get(),
        );
        self._logo_was_docked.set_if_neq(docked);
        match action {
            LogoReturnAction::Stage => {
                // Bring the initial pose home before unfurling, so none of the
                // reveal is spent offscreen during the reverse portal trip.
                self._logo_return_pending.set(true);
                self.playhead_ms.set(0.0);
                self._light_last_ms.set(now);
            }
            LogoReturnAction::Replay => {
                self._logo_return_pending.set(false);
                self.playhead_ms.set(0.0);
                self._logo_started.set(true);
                self._light_last_ms.set(now);
            }
            LogoReturnAction::Cancel => {
                // A reversal or source drawer interruption must not launch a
                // new intro while the logo is leaving for the navigation.
                self._logo_return_pending.set(false);
                self.playhead_ms.set(2100.0);
                self._light_last_ms.set(now);
            }
            LogoReturnAction::None => {}
        }
    }

    fn bind_layout(&self, ctx: &NodeContext) {
        crate::section_layout::bind_compact(ctx, &self.compact, 900.0);
        let width = ctx.bounds_self.clone();
        let deps = [width.untyped()];
        let font_px = self.font_px.local();
        let logo_width = self.logo_width.local();
        let vertical_padding = self._vertical_padding.local();
        let update = move || {
            let width = width.get().0;
            // Top inset plus bottom breathing room; desktop copy starts 56px lower.
            vertical_padding.set_if_neq(if width < 900.0 { 104.0 } else { 160.0 });
            let text_width = if width < 900.0 {
                width - 48.0
            } else {
                width * 0.88 * 0.66
            };
            // Reserve the widest first line (Build eye-catching) for every word.
            font_px.set_if_neq((text_width / 10.4).clamp(23.0, 90.0));
            let target_logo_width = if width < 900.0 {
                // Reserve View Source's 112px width and a 24px gap on the shared top row.
                ((width - 48.0).min(300.0) * 0.8).min((width - 184.0).max(1.0))
            } else {
                width * 0.88 * 0.28
            };
            logo_width.set_if_neq(target_logo_width);
        };
        update();
        ctx.subscribe(&deps, update);
    }

    pub fn bind_logo_portal(&mut self, ctx: &NodeContext) {
        // get_node_interface() addresses the containing component; this inline
        // handler needs the home Group itself, whose footprint stays put.
        let Some(home) = ctx.expanded_node.upgrade().map(NodeInterface::from) else {
            return;
        };
        let Ok(nav) = ctx.with_store(|nav: &mut LogoNavigation| nav.clone()) else {
            return;
        };
        let geometry = home.transform_and_bounds();
        let scroll_y = self.scroll_y.local();
        let ready = self._portal_ready.local();
        let docked = self.logo_docked.local();
        let lift = self.portal_lift.local();
        let deps = [
            geometry.untyped(),
            scroll_y.untyped(),
            nav.nav_bottom.untyped(),
            nav.hidden.local().untyped(),
            ready.untyped(),
        ];
        ctx.subscribe(&deps, move || {
            if !ready.get() {
                return;
            }
            let home = geometry.get();
            if home.bounds.0 <= 0.0 || home.bounds.1 <= 0.0 {
                return;
            }
            // Measure the reserved home, not the animated child, so the portal
            // cannot feed back into its own threshold or alter section layout.
            let home_top = home
                .corners()
                .iter()
                .map(|p| p.y)
                .fold(f64::INFINITY, f64::min);
            // Include a generous painted-outset margin, not just the glyph box.
            // This remains fully above the viewport even if scrolling reverses to 0.
            lift.set_if_neq(home_top.max(0.0) + home.bounds.1 * 1.4 + 96.0);
            // Main's compact hero starts inside the future nav's bounds. Leave
            // enough departure distance for the 24px return hysteresis at rest.
            let boundary = nav.nav_bottom.get().min(home_top - 32.0);
            let hidden = nav.hidden.get();
            let next = !hidden && should_dock(home_top - scroll_y.get(), boundary, docked.get());
            if next == docked.get() {
                return;
            }
            docked.set(next);
            nav.hero_docked.local().set(next);
        });
    }

    pub fn on_unmount(&mut self, ctx: &NodeContext) {
        if let Ok(present) = ctx.with_store(|nav: &mut LogoNavigation| nav.hero_present.clone()) {
            present.local().set(false);
        }
        if let Ok(docked) = ctx.with_store(|nav: &mut LogoNavigation| nav.hero_docked.clone()) {
            docked.local().set(false);
        }
    }
}

fn should_dock(home_top: f64, nav_bottom: f64, docked: bool) -> bool {
    home_top.is_finite()
        && nav_bottom.is_finite()
        && home_top <= nav_bottom + if docked { 24.0 } else { 0.0 }
}

#[derive(Debug, PartialEq)]
enum LogoReturnAction {
    None,
    Stage,
    Replay,
    Cancel,
}

fn logo_return_action(
    was_docked: bool,
    docked: bool,
    hidden: bool,
    pending: bool,
    departure: f64,
) -> LogoReturnAction {
    if hidden || docked {
        if pending {
            LogoReturnAction::Cancel
        } else {
            LogoReturnAction::None
        }
    } else if was_docked {
        LogoReturnAction::Stage
    } else if pending && departure <= 0.0 {
        LogoReturnAction::Replay
    } else {
        LogoReturnAction::None
    }
}

#[cfg(test)]
mod portal_tests {
    use super::*;

    #[test]
    fn compact_home_overlapping_nav_can_depart_and_return() {
        let home_top: f64 = 24.0;
        let boundary = 64.0_f64.min(home_top - 32.0);
        assert!(!should_dock(home_top, boundary, false));
        assert!(should_dock(home_top - 32.0, boundary, false));
        assert!(should_dock(home_top - 16.0, boundary, true));
        assert!(!should_dock(home_top, boundary, true));
    }

    #[test]
    fn hero_return_stages_then_replays_once_at_home() {
        use LogoReturnAction::*;
        assert_eq!(logo_return_action(false, false, false, false, 0.0), None);
        assert_eq!(logo_return_action(true, false, false, false, 1.0), Stage);
        assert_eq!(logo_return_action(false, false, false, true, 0.4), None);
        assert_eq!(logo_return_action(false, false, false, true, 0.0), Replay);
        assert_eq!(logo_return_action(false, false, false, false, 0.0), None);
    }

    #[test]
    fn nav_departures_and_source_inspection_do_not_replay_the_logo() {
        use LogoReturnAction::*;
        assert_eq!(logo_return_action(false, true, false, false, 0.0), None);
        assert_eq!(logo_return_action(true, true, false, false, 1.0), None);
        assert_eq!(logo_return_action(true, false, true, false, 0.0), None);
        assert_eq!(logo_return_action(false, true, false, true, 0.3), Cancel);
        assert_eq!(logo_return_action(false, false, true, true, 0.0), Cancel);
        assert!(include_str!("hero_section.pax").contains("!self._logo_return_pending"));
        assert!(include_str!("site_shell.pax").contains("progress=1 click_to_replay=false"));
    }
}
