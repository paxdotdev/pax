use pax_kit::*;

#[pax]
#[main]
#[file("lib.pax")]
pub struct Example {
    pub visible: Property<bool>,
}

impl Example {
    pub fn on_mount(&mut self, _ctx: &NodeContext) {
        self.visible.set(true);
    }

    pub fn toggle(&mut self, _ctx: &NodeContext, _event: Event<Click>) {
        self.visible.set(!self.visible.get());
    }
}

#[pax]
#[file("scope_probe.pax")]
pub struct ScopeProbe {
    pub label: Property<String>,
    pub visible: Property<bool>,
    pub mounts: Property<usize>,
    pub unmounts: Property<usize>,
    pub child_mounts: Property<usize>,
    pub child_unmounts: Property<usize>,
    pub tick_seen: Property<bool>,
    pub render_seen: Property<bool>,
    pub stack_render_seen: Property<bool>,
    pub target_width: Property<f64>,
}

impl ScopeProbe {
    pub fn card_mount(&mut self, ctx: &NodeContext) {
        self.mounts.set(self.mounts.get() + 1);
        self.target_width.set(ctx.bounds_self.get().0);
    }

    pub fn card_unmount(&mut self, _ctx: &NodeContext) {
        self.unmounts.set(self.unmounts.get() + 1);
    }

    pub fn card_tick(&mut self, _ctx: &NodeContext) {
        self.tick_seen.set(true);
    }

    pub fn card_pre_render(&mut self, _ctx: &NodeContext) {
        self.render_seen.set(true);
    }

    pub fn stack_pre_render(&mut self, _ctx: &NodeContext) {
        self.stack_render_seen.set(true);
    }

    pub fn child_mount(&mut self, _ctx: &NodeContext) {
        self.child_mounts.set(self.child_mounts.get() + 1);
    }

    pub fn child_unmount(&mut self, _ctx: &NodeContext) {
        self.child_unmounts.set(self.child_unmounts.get() + 1);
    }
}

#[pax]
#[inlined(
    <Text text={"Child settings mount: " + self.settings_mounts} width=100% height=100%
        style={ font_size: 16px, fill: rgb(112, 224, 189) } />
)]
pub struct LifecycleChild {
    pub settings_mounts: Property<usize>,
}

impl LifecycleChild {
    pub fn on_mount(&mut self, _ctx: &NodeContext) {
        self.settings_mounts.set(self.settings_mounts.get() + 1);
    }
}
