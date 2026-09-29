use pax_kit::*;

#[pax]
#[main]
#[file("lib.pax")]
pub struct Example {
    pub playing: Property<bool>,
    pub overlay: Property<bool>,
    pub compact: Property<bool>,
    pub panel_width: Property<f64>,
    pub field_height: Property<f64>,
    pub word_height: Property<f64>,
    pub center_x: Property<Size>,
    pub center_y: Property<Size>,
    pub accent: Property<Color>,
    pub rows: Property<Vec<Vec<MeshPoint>>>,
    pub elapsed: Property<f64>,
    pub last_time: Property<f64>,
    pub progress: Property<f64>,
}

impl Example {
    pub fn mount(&mut self, ctx: &NodeContext) {
        self.playing.set(true);
        self.overlay.set(true);
        self.last_time.set(ctx.elapsed_time_millis() as f64);
        self.update_field(0.0);
        self.tick(ctx);
    }

    pub fn tick(&mut self, ctx: &NodeContext) {
        let now = ctx.elapsed_time_millis() as f64;
        let dt = (now - self.last_time.get()).clamp(0.0, 50.0);
        self.last_time.set(now);
        let max_width = if self.compact.get() { 390.0 } else { 920.0 };
        let width = (ctx.viewport.get().width - 40.0).clamp(200.0, max_width);
        self.panel_width.set_if_neq(width);
        self.field_height
            .set_if_neq((width * 0.44).clamp(200.0, 340.0));
        self.word_height
            .set_if_neq((width * 0.2).clamp(64.0, 160.0));
        if self.playing.get() {
            let elapsed = self.elapsed.get() + dt;
            self.elapsed.set(elapsed);
            self.update_field(elapsed / 1000.0);
            self.progress
                .set((elapsed.rem_euclid(9000.0) / 3600.0).min(1.0));
        }
    }

    fn update_field(&self, time: f64) {
        let x = 50.0 + 17.0 * (time * 0.53).sin();
        let y = 50.0 + 18.0 * (time * 0.41 + 0.4).sin();
        let accent = Color::CYAN.interpolate(&Color::FUCHSIA, (time * 0.31).sin() * 0.5 + 0.5);
        self.center_x.set(Size::Percent(x.into()));
        self.center_y.set(Size::Percent(y.into()));
        self.accent.set(accent.clone());
        let colors = [
            Color::CYAN,
            Color::FUCHSIA,
            Color::YELLOW,
            Color::FUCHSIA,
            accent,
            Color::CYAN,
            Color::YELLOW,
            Color::CYAN,
            Color::FUCHSIA,
        ];
        self.rows.set(
            (0..3)
                .map(|row| {
                    (0..3)
                        .map(|column| MeshPoint {
                            position: (
                                Size::Percent(
                                    (if row == 1 && column == 1 {
                                        x
                                    } else {
                                        column as f64 * 50.0
                                    })
                                    .into(),
                                ),
                                Size::Percent(
                                    (if row == 1 && column == 1 {
                                        y
                                    } else {
                                        row as f64 * 50.0
                                    })
                                    .into(),
                                ),
                            ),
                            color: colors[row * 3 + column].clone(),
                        })
                        .collect()
                })
                .collect(),
        );
    }

    pub fn toggle_playing(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.playing.set(!self.playing.get());
    }
    pub fn toggle_overlay(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.overlay.set(!self.overlay.get());
    }
    pub fn toggle_size(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.compact.set(!self.compact.get());
    }
}
