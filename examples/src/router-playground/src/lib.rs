#![allow(unused_imports)]

use pax_kit::*;

pub mod guide_panel;
pub mod landing_panel;
pub mod mobile_menu_drawer;
pub mod mobile_menu_underlay;
pub mod route_inspector;
pub mod route_outlet;
pub mod route_transition_frame;
pub mod sidebar_nav;
pub mod team_panel;
pub mod top_level_fallback_panel;

pub use guide_panel::GuidePanel;
pub use landing_panel::LandingPanel;
pub use mobile_menu_drawer::MobileMenuDrawer;
pub use mobile_menu_underlay::MobileMenuUnderlay;
pub use route_inspector::RouteInspector;
pub use route_outlet::RouteOutlet;
pub use route_transition_frame::RouteTransitionFrame;
pub use sidebar_nav::SidebarNav;
pub use team_panel::TeamPanel;
pub use top_level_fallback_panel::TopLevelFallbackPanel;

const MOBILE_BREAKPOINT_WIDTH: f64 = 920.0;

#[pax]
#[main]
#[file("lib.pax")]
pub struct Example {
    pub is_mobile: Property<bool>,
    pub mobile_menu_open: Property<bool>,
    pub mobile_top_inset: Property<f64>,
}

pub struct RouterPlaygroundChromeStore {
    pub mobile_menu_open: Property<bool>,
}

impl Store for RouterPlaygroundChromeStore {}

impl Example {
    pub fn handle_mount(&mut self, ctx: &NodeContext) {
        self.mobile_top_inset
            .set(if ctx.os.is_ios() { 56.0 } else { 0.0 });
        ctx.push_local_store(RouterPlaygroundChromeStore {
            mobile_menu_open: self.mobile_menu_open.clone(),
        });
        self.sync_responsive_state(ctx);
    }

    pub fn handle_pre_render(&mut self, ctx: &NodeContext) {
        self.sync_responsive_state(ctx);
    }

    pub fn toggle_mobile_menu(&mut self, _ctx: &NodeContext, _args: Event<Click>) {
        self.mobile_menu_open.set(!self.mobile_menu_open.get());
    }

    pub fn close_mobile_menu(&mut self, _ctx: &NodeContext, _args: Event<Click>) {
        self.mobile_menu_open.set(false);
    }

    fn sync_responsive_state(&mut self, ctx: &NodeContext) {
        let (width, _) = ctx.bounds_self.get();
        let is_mobile = width < MOBILE_BREAKPOINT_WIDTH;
        self.is_mobile.set_if_neq(is_mobile);
        if !is_mobile {
            self.mobile_menu_open.set_if_neq(false);
        }
    }
}
