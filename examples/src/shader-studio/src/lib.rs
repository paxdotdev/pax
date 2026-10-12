use pax_kit::*;

#[pax]
#[main]
#[file("lib.pax")]
pub struct Example {
    pub mode: Property<ImpressionistMode>,
    pub mode_description: Property<String>,
    pub brush_size: Property<f64>,
    pub brush_sizes: Property<Vec<f64>>,
    pub motion_speed: Property<f64>,
    pub phase: Property<f64>,
    pub last_time: Property<f64>,
    pub playing: Property<bool>,
    pub moving: Property<bool>,
    pub source_phase: Property<f64>,
    pub radius: Property<f64>,
    pub reference: Property<bool>,
    pub panel_width: Property<f64>,
}
impl Example {
    pub fn mount(&mut self, ctx: &NodeContext) {
        self.brush_size.set(24.);
        self.brush_sizes.set(vec![24.; 6]);
        self.motion_speed.set(1.);
        self.set_mode(ImpressionistMode::Basic);
        self.playing.set(true);
        self.radius.set(8.);
        self.last_time.set(ctx.elapsed_time_millis() as f64);
    }
    pub fn tick(&mut self, ctx: &NodeContext) {
        let now = ctx.elapsed_time_millis() as f64;
        let dt = (now - self.last_time.get()).clamp(0., 50.) / 1000.;
        self.last_time.set(now);
        if self.playing.get() {
            self.phase
                .set(self.phase.get() + dt * self.motion_speed.get());
        }
        if self.moving.get() {
            self.source_phase.set(self.source_phase.get() + dt);
        }
        self.panel_width
            .set_if_neq((ctx.viewport.get().width - 48.).clamp(240., 1040.));
    }
    pub fn toggle_play(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.playing.set(!self.playing.get());
    }
    pub fn toggle_source(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.moving.set(!self.moving.get());
    }
    pub fn toggle_reference(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.reference.set(!self.reference.get());
    }
    pub fn blur_more(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.radius.set((self.radius.get() + 8.).rem_euclid(56.));
    }
    fn set_mode(&mut self, mode: ImpressionistMode) {
        let mut sizes = self.brush_sizes.get();
        sizes[mode_index(&self.mode.get())] = self.brush_size.get();
        self.brush_size.set(sizes[mode_index(&mode)]);
        self.brush_sizes.set(sizes);
        self.mode_description.set(match mode {
            ImpressionistMode::Basic => "Basic — soft, layered marks in three sizes. The original living surface.",
            ImpressionistMode::Gauguin => "Gauguin — broad sweeps, flowing planes of color, and quiet bristle texture.",
            ImpressionistMode::VanGogh => "Van Gogh — drifting, bending currents and ridges of light and shadow in thick paint.",
            ImpressionistMode::Signac => "Signac — fine, closely packed dabs with pale and vivid pigment woven together.",
            ImpressionistMode::Cezanne => "Cézanne — varied planes of strokes, nested arcs, and wispy outlines.",
            ImpressionistMode::Seurat => "Seurat — dense, rounded points of color that gather into a luminous surface.",
        }.into());
        self.mode.set(mode);
    }
    pub fn basic(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.set_mode(ImpressionistMode::Basic);
    }
    pub fn gauguin(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.set_mode(ImpressionistMode::Gauguin);
    }
    pub fn van_gogh(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.set_mode(ImpressionistMode::VanGogh);
    }
    pub fn signac(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.set_mode(ImpressionistMode::Signac);
    }
    pub fn cezanne(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.set_mode(ImpressionistMode::Cezanne);
    }
    pub fn seurat(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.set_mode(ImpressionistMode::Seurat);
    }
}

fn mode_index(mode: &ImpressionistMode) -> usize {
    match mode {
        ImpressionistMode::Basic => 0,
        ImpressionistMode::Gauguin => 1,
        ImpressionistMode::VanGogh => 2,
        ImpressionistMode::Signac => 3,
        ImpressionistMode::Cezanne => 4,
        ImpressionistMode::Seurat => 5,
    }
}

#[pax]
#[file("landscape.pax")]
#[has_helpers]
pub struct Landscape {
    pub phase: Property<f64>,
}

#[helpers]
impl Landscape {
    pub fn sun_x(phase: f64) -> f64 {
        50.0 + 14.0 * phase.sin()
    }
}
