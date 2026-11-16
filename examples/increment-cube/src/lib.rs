use pax_kit::*;
use rand::Rng;
mod input;
mod motion;

pax_kit::runnable! { root: crate::Example }

#[pax]
#[file("lib.pax")]
pub struct Example {}

#[pax]
pub struct CubeFace {
    pub side: CuboidSide,
    pub label: String,
    pub label_opacity: f64,
}

/// A finite, click-driven roll: template bindings expose its pose and count.
#[pax]
#[file("increment.pax")]
pub struct IncrementCube {
    pub count: Property<u64>,
    pub count_font_size: Property<f64>,
    pub faces: Property<Vec<CubeFace>>,
    pub rolling: Property<bool>,
    pub horizontal: Property<bool>,
    pub scale: Property<f64>,
    pub pulse: Property<f64>,
    pub hint_jump: Property<f64>,
    pub hint_stretch: Property<f64>,
    pub hint_opacity: Property<f64>,
    pub rotation_x: Property<f64>,
    pub rotation_y: Property<f64>,
    pub rotation_z: Property<f64>,
    #[pax(rust)]
    pub motion: motion::Roll,
    #[pax(rust)]
    pub history: motion::History,
    #[pax(rust)]
    pub poke: input::Poke,
    #[pax(rust)]
    pub hint_started_ms: f64,
}

impl IncrementCube {
    pub fn mount(&mut self, ctx: &NodeContext) {
        // Preserve later counts when a responsive layout remounts the cube.
        self.count.set_if_neq(self.count.get().max(6));
        let count = self.count.clone();
        let deps = [count.untyped()];
        self.count_font_size.replace_with(LocalProperty::computed(
            move || (270.0 / count.get().to_string().len().max(2) as f64).min(106.0),
            &deps,
        ));
        self.pulse.set(1.0);
        self.hint_started_ms = ctx.elapsed_time_millis() as f64;
        self.hint_jump.set(0.0);
        self.hint_stretch.set(1.0);
        self.hint_opacity
            .set(if self.count.get() == 6 { 1.0 } else { 0.0 });
        self.history = motion::History::new(self.count.get());
        self.publish_faces();
        let bounds = ctx.bounds_self.clone();
        let deps = [bounds.untyped()];
        self.horizontal.replace_with(LocalProperty::computed(
            move || {
                let (width, height) = bounds.get();
                width > height
            },
            &deps,
        ));
        let bounds = ctx.bounds_self.clone();
        self.scale.replace_with(LocalProperty::computed(
            move || {
                let (width, height) = bounds.get();
                // Fit the pair to its own container, including space for the roll.
                let (scene_width, scene_height) = if width > height {
                    (600.0, 400.0)
                } else {
                    (400.0, 500.0)
                };
                (width / scene_width).min(height / scene_height).min(1.25) * 100.0
            },
            &deps,
        ));
        self.pose([0.0; 3]);
    }

    fn roll(&mut self, ctx: &NodeContext) {
        let Some(count) = self.count.get().checked_add(1) else {
            return;
        };
        self.count.set(count);
        if count == 7 {
            self.hint_opacity.local().ease_to(
                0.0,
                Duration::Milliseconds(360.into()),
                EasingCurve::InOutQuad,
            );
        }
        let mut rng = rand::thread_rng();
        let face = self.history.record(count, rng.gen());
        self.publish_faces();
        let turns = std::array::from_fn(|_| if rng.gen() { 1 } else { -1 });
        self.motion
            .wind(ctx.elapsed_time_millis() as f64, face, turns);
        self.rolling.set(true);
        self.tick(ctx);
    }

    pub fn tick(&mut self, ctx: &NodeContext) {
        if !self.rolling.get() && self.hint_opacity.get() <= 0.0 {
            return;
        }
        let now = ctx.elapsed_time_millis() as f64;
        if self.hint_opacity.get() > 0.0 {
            let (jump, stretch) = motion::hint_pose(now - self.hint_started_ms);
            self.hint_jump.set_if_neq(jump);
            self.hint_stretch.set_if_neq(stretch);
        }
        if self.rolling.get() {
            self.pose(self.motion.sample(now).0);
            self.rolling.set(!self.motion.finished(now));
        }
    }

    fn pose(&mut self, angles: [f64; 3]) {
        self.rotation_x.set_if_neq(angles[0].rem_euclid(360.0));
        self.rotation_y.set_if_neq(angles[1].rem_euclid(360.0));
        self.rotation_z.set_if_neq(angles[2].rem_euclid(360.0));
        let facing = motion::facing(angles);
        // Keep native labels mounted for input capture, but omit rear-facing ink.
        self.faces.update(|faces| {
            for (face, z) in faces.iter_mut().zip(facing) {
                face.label_opacity = if z > 0.001 { 1.0 } else { 0.0 };
            }
        });
    }

    fn publish_faces(&mut self) {
        self.faces.set(
            [
                CuboidSide::Front,
                CuboidSide::Back,
                CuboidSide::Right,
                CuboidSide::Left,
                CuboidSide::Top,
                CuboidSide::Bottom,
            ]
            .into_iter()
            .zip(self.history.numbers)
            .map(|(side, number)| CubeFace {
                side,
                label: number.map(|n| n.to_string()).unwrap_or_default(),
                label_opacity: 1.0,
            })
            .collect(),
        );
    }

    pub fn down(&mut self, _: &NodeContext, event: Event<MouseDown>) {
        if matches!(event.mouse.button, MouseButton::Left) {
            self.press(None, event.mouse.x, event.mouse.y);
        }
    }

    pub fn up(&mut self, ctx: &NodeContext, event: Event<MouseUp>) {
        if matches!(event.mouse.button, MouseButton::Left) {
            self.lift(ctx, None, event.mouse.x, event.mouse.y);
        }
    }

    pub fn mouse_move(&mut self, _: &NodeContext, event: Event<MouseMove>) {
        if self.poke.moved(None, event.mouse.x, event.mouse.y) {
            self.relax();
        }
    }
    pub fn mouse_cancel(&mut self, _: &NodeContext, event: Event<MouseCancel>) {
        if event.mouse.button == MouseButton::Left && self.poke.cancel(None) {
            self.relax();
        }
    }
    pub fn touch_start(&mut self, _: &NodeContext, event: Event<TouchStart>) {
        if let Some(touch) = event.touches.first() {
            self.press(Some(touch.identifier), touch.x, touch.y);
        }
    }
    pub fn touch_move(&mut self, _: &NodeContext, event: Event<TouchMove>) {
        for touch in &event.touches {
            if self.poke.moved(Some(touch.identifier), touch.x, touch.y) {
                self.relax();
            }
        }
    }
    pub fn touch_end(&mut self, ctx: &NodeContext, event: Event<TouchEnd>) {
        for touch in &event.touches {
            self.lift(ctx, Some(touch.identifier), touch.x, touch.y);
        }
    }
    pub fn touch_cancel(&mut self, _: &NodeContext, event: Event<TouchCancel>) {
        for touch in &event.touches {
            if self.poke.cancel(Some(touch.identifier)) {
                self.relax();
            }
        }
    }

    fn press(&mut self, id: Option<i64>, x: f64, y: f64) {
        if self.poke.begin(id, x, y) {
            self.compress();
        }
    }

    fn lift(&mut self, ctx: &NodeContext, id: Option<i64>, x: f64, y: f64) {
        if !self.poke.owns(id) {
            return;
        }
        if self.poke.end(id, x, y) {
            self.release();
            self.roll(ctx);
        } else {
            self.relax();
        }
    }

    fn compress(&mut self) {
        self.pulse.local().ease_to(
            0.88,
            Duration::Milliseconds(90.into()),
            EasingCurve::OutQuad,
        );
    }

    fn release(&mut self) {
        self.pulse.local().ease_to(
            1.10,
            Duration::Milliseconds(130.into()),
            EasingCurve::OutQuad,
        );
        self.pulse.local().ease_to_later(
            1.0,
            Duration::Milliseconds(230.into()),
            EasingCurve::InOutQuad,
        );
    }

    fn relax(&mut self) {
        self.pulse.local().ease_to(
            1.0,
            Duration::Milliseconds(160.into()),
            EasingCurve::OutQuad,
        );
    }
}

/// Complete authored source for the website's source viewer.
pub fn sources() -> Vec<ExampleSource> {
    [
        ("increment.pax", "pax", include_str!("increment.pax")),
        ("lib.rs", "rust", include_str!("lib.rs")),
        ("motion.rs", "rust", include_str!("motion.rs")),
        ("input.rs", "rust", include_str!("input.rs")),
        ("lib.pax", "pax", include_str!("lib.pax")),
        ("Cargo.toml", "toml", include_str!("../Cargo.toml")),
    ]
    .into_iter()
    .map(|(label, language, code)| ExampleSource {
        label: label.into(),
        language: language.into(),
        code: code.into(),
    })
    .collect()
}
