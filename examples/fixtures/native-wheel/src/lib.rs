use pax_kit::*;

pax_kit::runnable! { root: crate::NativeWheel }

#[pax]
#[file("lib.pax")]
pub struct NativeWheel {
    pub prevent_native_default: Property<bool>,
    pub outer_x: Property<f64>,
    pub outer_y: Property<f64>,
    pub inner_x: Property<f64>,
    pub inner_y: Property<f64>,
    pub root_events: Property<usize>,
    pub outer_events: Property<usize>,
    pub inner_events: Property<usize>,
    pub overlay_events: Property<usize>,
    pub counts: Property<String>,
    pub positions: Property<String>,
    pub last_event: Property<String>,
}

impl NativeWheel {
    pub fn on_mount(&mut self, _: &NodeContext) {
        self.reset_values();
    }

    fn reset_values(&mut self) {
        self.prevent_native_default.set(false);
        self.outer_x.set(0.0);
        self.outer_y.set(0.0);
        self.inner_x.set(0.0);
        self.inner_y.set(0.0);
        self.root_events.set(0);
        self.outer_events.set(0);
        self.inner_events.set(0);
        self.overlay_events.set(0);
        self.last_event.set("No wheel input yet".into());
    }

    pub fn reset(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.reset_values();
    }

    pub fn inner_wheel(&mut self, _: &NodeContext, _: Event<Wheel>) {
        self.inner_events.set(self.inner_events.get() + 1);
    }

    pub fn outer_wheel(&mut self, _: &NodeContext, _: Event<Wheel>) {
        self.outer_events.set(self.outer_events.get() + 1);
    }

    pub fn overlay_wheel(&mut self, _: &NodeContext, _: Event<Wheel>) {
        // This sibling is outside both Scrollers. Leave default prevention to
        // the shared hit/scroll-owner routing, so the fixture can detect mistakes.
        self.overlay_events.set(self.overlay_events.get() + 1);
    }

    pub fn root_wheel(&mut self, _: &NodeContext, event: Event<Wheel>) {
        self.root_events.set(self.root_events.get() + 1);
        let prevented = self.prevent_native_default.get();
        if prevented {
            // Cancellation at the ancestor must preserve delivery to each handler.
            event.prevent_default();
        }
        let modifiers = event
            .modifiers
            .iter()
            .map(|modifier| match modifier {
                ModifierKey::Shift => "Shift",
                ModifierKey::Control => "Control",
                ModifierKey::Alt => "Alt",
                ModifierKey::Command => "Command",
            })
            .collect::<Vec<_>>()
            .join("+");
        self.last_event.set(format!(
            "raw dx={:.3} dy={:.3} at ({:.1}, {:.1}) modifiers={} prevented={}",
            event.delta_x,
            event.delta_y,
            event.x,
            event.y,
            if modifiers.is_empty() {
                "none"
            } else {
                &modifiers
            },
            prevented,
        ));
    }

    pub fn pre_render(&mut self, _: &NodeContext) {
        self.counts.set_if_neq(format!(
            "Events  inner={}  outer={}  root={}  overlay={}",
            self.inner_events.get(),
            self.outer_events.get(),
            self.root_events.get(),
            self.overlay_events.get(),
        ));
        self.positions.set_if_neq(format!(
            "Offsets  inner=({:.1}, {:.1})  outer=({:.1}, {:.1})",
            self.inner_x.get(),
            self.inner_y.get(),
            self.outer_x.get(),
            self.outer_y.get(),
        ));
    }

    pub fn snapshot(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        let line = format!(
            "pax-native-wheel: inner_events={} outer_events={} root_events={} overlay_events={} inner_x={:.3} inner_y={:.3} outer_x={:.3} outer_y={:.3} prevent_native_default={} last=[{}]",
            self.inner_events.get(), self.outer_events.get(), self.root_events.get(), self.overlay_events.get(),
            self.inner_x.get(), self.inner_y.get(), self.outer_x.get(), self.outer_y.get(),
            self.prevent_native_default.get(), self.last_event.get(),
        );
        eprintln!("{line}");
        // Windows GUI cartridges may not inherit stderr. Keep packaged Release
        // observations available without enabling development endpoints.
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(path) = std::env::var_os("PAX_NATIVE_WHEEL_LOG") {
            use std::io::Write;
            let mut log = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
                .expect("open PAX_NATIVE_WHEEL_LOG");
            writeln!(log, "{line}").expect("write PAX_NATIVE_WHEEL_LOG");
        }
    }
}
