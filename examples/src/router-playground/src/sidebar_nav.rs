#![allow(unused_imports)]

use pax_kit::*;

const DEMO_ROUTES: [&str; 4] = [
    "/guide/topic/router?view=api#bindings",
    "/guide/playground/layers?view=trace#tail",
    "/teams/design/members/ada?lane=beta#inspect",
    "/teams/ops/settings/integrations/logs?source=router#history",
];

#[pax]
#[file("sidebar_nav.pax")]
pub struct SidebarNav {
    pub jump_index: Property<usize>,
    pub mobile_menu_open: Property<bool>,
    pub next_jump_target: Property<String>,
}

impl SidebarNav {
    pub fn handle_mount(&mut self, _ctx: &NodeContext) {
        self.sync_next_jump_target();
    }

    pub fn jump_to_next_demo(&mut self, ctx: &NodeContext, _args: Event<Click>) {
        let index = self.jump_index.get() % DEMO_ROUTES.len();
        ctx.navigate_to(DEMO_ROUTES[index], NavigationTarget::Current);
        self.jump_index.set((index + 1) % DEMO_ROUTES.len());
        self.sync_next_jump_target();
        self.set_mobile_menu_closed();
    }

    pub fn close_mobile_menu_after_link(&mut self, _ctx: &NodeContext, _args: Event<ClickOrTap>) {
        self.set_mobile_menu_closed();
    }

    fn sync_next_jump_target(&mut self) {
        let index = self.jump_index.get() % DEMO_ROUTES.len();
        self.next_jump_target.set(DEMO_ROUTES[index].to_string());
    }

    fn set_mobile_menu_closed(&mut self) {
        self.mobile_menu_open.set_if_neq(false);
    }
}
