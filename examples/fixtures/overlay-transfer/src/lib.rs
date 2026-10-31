use pax_kit::*;
mod fixed_nav;
mod identity_logo;
use fixed_nav::FixedNav;
use identity_logo::IdentityLogo;

pax_kit::runnable! { root: crate::Example }

#[pax]
#[custom(Default)]
#[file("lib.pax")]
pub struct Example {
    pub scroll_y: Property<f64>,
    pub show_nav: Property<bool>,
    pub show_source: Property<bool>,
    pub responsible: Property<bool>,
    pub automatic: Property<bool>,
    pub docked: Property<bool>,
    pub detached_count: Property<usize>,
    pub status: Property<String>,
}

impl Default for Example {
    fn default() -> Self {
        Self {
            scroll_y: Property::new(0.),
            show_nav: Property::new(true),
            show_source: Property::new(true),
            responsible: Property::new(true),
            automatic: Property::new(true),
            docked: Property::new(false),
            detached_count: Property::new(0),
            status: Property::new("Scroll to send the same logo into the fixed nav.".into()),
        }
    }
}

impl Example {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        let state = self.clone();
        let context = ctx.clone();
        ctx.subscribe(
            &[
                self.scroll_y.untyped(),
                self.automatic.untyped(),
                self.show_source.untyped(),
                self.show_nav.untyped(),
                self.detached_count.untyped(),
                self.docked.untyped(),
            ],
            move || state.sync_scroll_transfer(&context),
        );
    }

    fn transfer(&self, ctx: &NodeContext, dock: bool) -> Result<(), NodeError> {
        let root = ctx.app_root()?;
        let logo = root.query("#logo").one()?;
        let movement = logo.animated(Duration::Milliseconds(320.into()), EasingCurve::OutQuad);
        if dock {
            let destination = root.query("#nav").query("#logo_container").one()?;
            movement.move_to_append_child(&destination)?;
        } else {
            movement.move_to_home()?;
        }
        self.docked.set(dock);
        self.status.set(
            if dock {
                "The original logo is visiting the nav."
            } else {
                "The same mounted logo is home."
            }
            .into(),
        );
        Ok(())
    }
    pub fn on_tick(&mut self, ctx: &NodeContext) {
        let count = ctx.detached_nodes().len().unwrap_or(0);
        self.detached_count.set(count);
    }

    fn sync_scroll_transfer(&self, ctx: &NodeContext) {
        if !self.automatic.get()
            || !self.show_source.get()
            || !self.show_nav.get()
            || self.detached_count.get() > 0
        {
            return;
        }
        // The home starts at 338px and the nav ends at 104px. Start as their
        // edges meet, with 24px of hysteresis to avoid jitter near that boundary.
        let dock = self.scroll_y.get() >= if self.docked.get() { 210. } else { 234. };
        if dock != self.docked.get() {
            if let Err(error) = self.transfer(ctx, dock) {
                self.status.set(error.to_string());
            }
        }
    }
    pub fn move_logo(&mut self, ctx: &NodeContext, _: Event<ButtonClick>) {
        self.automatic.set(false);
        if let Err(error) = self.transfer(ctx, !self.docked.get()) {
            self.status.set(error.to_string());
        }
    }
    pub fn toggle_nav(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.automatic.set(false);
        self.show_nav.set(!self.show_nav.get());
        self.docked.set(false);
        self.status.set(
            if self.show_nav.get() {
                "Nav remounted; transfers are explicit."
            } else if self.responsible.get() {
                "Nav cleanup returned its visitor before unmount."
            } else {
                "Nav abandoned its visitor. Recover it to start a fresh mount."
            }
            .into(),
        );
    }
    pub fn toggle_source(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.automatic.set(false);
        self.show_source.set(!self.show_source.get());
        self.docked.set(false);
        self.status.set(
            if self.show_source.get() {
                "The original declaration created a new logo at home."
            } else {
                "The original declaration ended: its logo and recovery entry are gone."
            }
            .into(),
        );
    }
    pub fn toggle_responsible(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.responsible.set(!self.responsible.get());
    }
    pub fn recover(&mut self, ctx: &NodeContext, _: Event<ButtonClick>) {
        self.automatic.set(false);
        let result = ctx.detached_nodes().all().and_then(|nodes| {
            for node in nodes {
                node.move_to_home()?;
            }
            Ok(())
        });
        self.docked.set(false);
        self.status.set(match result {
            Ok(()) => "Recovered at home with fresh private state. Scroll up to inspect it.".into(),
            Err(error) => error.to_string(),
        });
    }
    pub fn toggle_automatic(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.automatic.set(!self.automatic.get());
    }
}
