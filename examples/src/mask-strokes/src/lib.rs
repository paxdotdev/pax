use pax_kit::*;

#[pax]
#[main]
#[file("lib.pax")]
pub struct Example {
    pub playing: Property<bool>,
    pub mounted: Property<bool>,
    pub panel_width: Property<f64>,
    pub word_height: Property<f64>,
    pub band_x: Property<f64>,
    pub elapsed: Property<f64>,
    pub last_time: Property<f64>,
}

impl Example {
    pub fn mount(&mut self, ctx: &NodeContext) {
        self.playing.set(true);
        self.mounted.set(true);
        self.last_time.set(ctx.elapsed_time_millis() as f64);
        self.tick(ctx);
    }

    pub fn tick(&mut self, ctx: &NodeContext) {
        let now = ctx.elapsed_time_millis() as f64;
        let dt = (now - self.last_time.get()).clamp(0.0, 50.0);
        self.last_time.set(now);
        let width = (ctx.viewport.get().width - 48.0).clamp(220.0, 760.0);
        self.panel_width.set_if_neq(width);
        self.word_height.set_if_neq((width - 48.0) / 4.979216);
        if self.playing.get() {
            self.elapsed.set(self.elapsed.get() + dt);
            self.band_x
                .set((self.elapsed.get() / 40.0).rem_euclid(100.0) - 100.0);
        }
    }

    pub fn toggle_playing(&mut self, _ctx: &NodeContext, _event: Event<Click>) {
        self.playing.set(!self.playing.get());
    }

    pub fn toggle_mounted(&mut self, _ctx: &NodeContext, _event: Event<Click>) {
        self.mounted.set(!self.mounted.get());
    }
}

/// The source owns its writing clock, proving that mask component lifecycle
/// handlers remain live independently of the content animation underneath it.
#[pax]
#[file("writing_source.pax")]
pub struct WritingSource {
    pub playing: Property<bool>,
    pub progress: Property<f64>,
    pub elapsed: Property<f64>,
    pub last_time: Property<f64>,
}

impl WritingSource {
    pub fn mount(&mut self, ctx: &NodeContext) {
        self.last_time.set(ctx.elapsed_time_millis() as f64);
    }

    pub fn tick(&mut self, ctx: &NodeContext) {
        let now = ctx.elapsed_time_millis() as f64;
        let dt = (now - self.last_time.get()).clamp(0.0, 50.0);
        self.last_time.set(now);
        if self.playing.get() {
            self.elapsed
                .set((self.elapsed.get() + dt).rem_euclid(6500.0));
            self.progress.set((self.elapsed.get() / 2800.0).min(1.0));
        }
    }
}
