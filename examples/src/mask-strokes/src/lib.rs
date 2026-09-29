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

    pub fn toggle_playing(&mut self, _ctx: &NodeContext, _event: Event<ButtonClick>) {
        self.playing.set(!self.playing.get());
    }

    pub fn toggle_mounted(&mut self, _ctx: &NodeContext, _event: Event<ButtonClick>) {
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

/// Exercises nested-property edits without replacing the outer layer vector.
#[pax]
#[file("layer_studies.pax")]
pub struct LayerStudies {
    pub accent: Property<Color>,
    pub strokes: Property<Vec<Stroke>>,
    pub removed: Property<bool>,
    pub progress: Property<f64>,
    pub running: Property<bool>,
    pub elapsed: Property<f64>,
    pub last_time: Property<f64>,
}

impl LayerStudies {
    pub fn mount(&mut self, ctx: &NodeContext) {
        self.running.set(true);
        self.last_time.set(ctx.elapsed_time_millis() as f64);
        self.reset_strokes();
    }

    fn reset_strokes(&self) {
        self.strokes.set(vec![
            Stroke {
                paint: Property::new(Color::WHITE.into()),
                width: Property::new(Size::Pixels(3.into())),
                ..Default::default()
            },
            Stroke {
                paint: Property::new(Color::CYAN.into()),
                width: Property::new(Size::Pixels(16.into())),
                cap: Property::new(StrokeCap::Round),
                join: Property::new(StrokeJoin::Round),
                ..Default::default()
            },
        ]);
    }

    pub fn tick(&mut self, ctx: &NodeContext) {
        let now = ctx.elapsed_time_millis() as f64;
        let dt = (now - self.last_time.get()).clamp(0.0, 50.0);
        self.last_time.set(now);
        if !self.running.get() {
            return;
        }
        let t = self.elapsed.get() + dt;
        self.elapsed.set(t);
        self.progress
            .set(0.25 + 0.75 * ((t / 2200.0).sin() * 0.5 + 0.5));
        self.accent
            .set(Color::CYAN.interpolate(&Color::FUCHSIA, (t / 1700.0).sin() * 0.5 + 0.5));
        // Only the nested paint changes. Width, geometry and the outer Vec stay live.
        for stroke in self.strokes.get() {
            if stroke.width.get().expect_pixels().to_float() > 4.0 {
                let start = Paint::linearGradient(
                    (Size::Percent(0.into()), Size::Percent(0.into())),
                    (Size::Percent(100.into()), Size::Percent(0.into())),
                    vec![
                        GradientStop::get(self.accent.get(), Size::Percent(0.into())),
                        GradientStop::get(Color::YELLOW, Size::Percent(100.into())),
                    ],
                );
                stroke.paint.set(start);
            }
        }
    }
    pub fn toggle(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.running.set(!self.running.get());
    }
    pub fn reorder(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        let mut layers = self.strokes.get();
        layers.reverse();
        self.strokes.set(layers);
    }
    pub fn remove(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        if self.strokes.get().is_empty() {
            self.reset_strokes();
            self.removed.set(false);
        } else {
            self.strokes.set(vec![]);
            self.removed.set(true);
        }
    }
}

/// Compares a subtree's ordinary appearance with the alpha of its retained capture.
#[pax]
#[file("capture_studies.pax")]
pub struct CaptureStudies {}

#[pax]
#[file("capture_pattern.pax")]
pub struct CapturePattern {}
