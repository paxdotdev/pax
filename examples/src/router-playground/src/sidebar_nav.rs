#![allow(unused_imports)]

use pax_kit::*;

use crate::RouterPlaygroundChromeStore;

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
    pub next_jump_target: Property<String>,
}

impl SidebarNav {
    pub fn handle_mount(&mut self, _ctx: &NodeContext) {
        self.sync_next_jump_target();
    }

    pub fn jump_to_next_demo(&mut self, ctx: &NodeContext, _args: Event<Click>) {
        let index = self.jump_index.get() % DEMO_ROUTES.len();
        self.navigate_and_close(ctx, DEMO_ROUTES[index]);
        self.jump_index.set((index + 1) % DEMO_ROUTES.len());
        self.sync_next_jump_target();
    }

    pub fn navigate_to_landing(&mut self, ctx: &NodeContext, _args: Event<Click>) {
        self.navigate_and_close(ctx, "/");
    }

    pub fn navigate_to_guide(&mut self, ctx: &NodeContext, _args: Event<Click>) {
        self.navigate_and_close(ctx, "/guide/topic/router?view=api#bindings");
    }

    pub fn navigate_to_team(&mut self, ctx: &NodeContext, _args: Event<Click>) {
        self.navigate_and_close(ctx, "/teams/design/members/ada?lane=beta#inspect");
    }

    pub fn navigate_to_not_found(&mut self, ctx: &NodeContext, _args: Event<Click>) {
        self.navigate_and_close(ctx, "/not-found/anywhere");
    }

    fn sync_next_jump_target(&mut self) {
        let index = self.jump_index.get() % DEMO_ROUTES.len();
        self.next_jump_target.set(DEMO_ROUTES[index].to_string());
    }

    fn navigate_and_close(&mut self, ctx: &NodeContext, url: &str) {
        self.set_mobile_menu_closed(ctx);
        ctx.navigate_to(url, NavigationTarget::Current);
    }

    fn set_mobile_menu_closed(&mut self, ctx: &NodeContext) {
        let _ = ctx.peek_local_store(|store: &mut RouterPlaygroundChromeStore| {
            store.mobile_menu_open.set_if_neq(false);
        });
    }
}
