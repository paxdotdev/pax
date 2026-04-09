#![allow(unused_imports)]

use pax_kit::*;

const BOARD_WIDTH: f64 = 980.0;
const BOARD_HEIGHT: f64 = 720.0;
const PLAYFIELD_WIDTH: f64 = 760.0;
const PLAYFIELD_HEIGHT: f64 = 430.0;
const STICK_COUNT: usize = 11;
const REMOVAL_FRAMES: u64 = 18;
const COLLAPSE_FRAMES: u64 = 42;
const LOCK_BUFFER_FRAMES: u64 = 4;

#[pax]
#[main]
#[file("lib.pax")]
pub struct PickUpSticks {
    pub sticks: Property<Vec<Stick>>,
    pub board_scale: Property<f64>,
    pub locked: Property<bool>,
    pub finished: Property<bool>,
    pub collapsed: Property<bool>,
    pub removed_count: Property<usize>,
    pub unlock_frame: Property<u64>,
    pub status_text: Property<String>,
    pub chance_text: Property<String>,
    pub progress_text: Property<String>,
    pub rebuild_taps: Property<usize>,
    pub rebuild_pulse: Property<f64>,
    pub rebuild_caption: Property<String>,
    pub seed: Property<u64>,
}

#[pax]
#[custom(Interpolatable)]
pub struct Stick {
    pub x: f64,
    pub y: f64,
    pub length: f64,
    pub rotation: f64,
    pub opacity: f64,
    pub thickness: f64,
    pub scale: f64,
    pub color: Color,
    pub highlight_stroke: Stroke,
    pub main_stroke: Stroke,
    pub shadow_stroke: Stroke,
    pub active: bool,
}

impl Interpolatable for Stick {
    fn interpolate(&self, other: &Self, t: f64) -> Self {
        Self {
            x: self.x.interpolate(&other.x, t),
            y: self.y.interpolate(&other.y, t),
            length: self.length.interpolate(&other.length, t),
            rotation: self.rotation.interpolate(&other.rotation, t),
            opacity: self.opacity.interpolate(&other.opacity, t),
            thickness: self.thickness.interpolate(&other.thickness, t),
            scale: self.scale.interpolate(&other.scale, t),
            color: self.color.interpolate(&other.color, t),
            highlight_stroke: self.highlight_stroke.interpolate(&other.highlight_stroke, t),
            main_stroke: self.main_stroke.interpolate(&other.main_stroke, t),
            shadow_stroke: self.shadow_stroke.interpolate(&other.shadow_stroke, t),
            active: self.active,
        }
    }
}

impl PickUpSticks {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        let bounds_parent = ctx.bounds_parent.clone();
        self.board_scale.replace_with(Property::computed(
            move || {
                let (width, height) = bounds_parent.get();
                (((width - 40.0) / BOARD_WIDTH).min((height - 40.0) / BOARD_HEIGHT))
                    .clamp(0.52, 1.0)
            },
            &[ctx.bounds_parent.untyped()],
        ));

        self.rebuild_taps.set(0);
        self.rebuild_pulse.set(0.0);
        self.rebuild_caption.set(String::from("tap to reset"));
        self.reseed(ctx);
        self.reset_state();
    }

    pub fn tick(&mut self, ctx: &NodeContext) {
        if self.rebuild_pulse.get() > 0.0 {
            self.rebuild_pulse
                .set((self.rebuild_pulse.get() - 0.12).max(0.0));
        }

        if self.locked.get()
            && !self.finished.get()
            && ctx.frames_elapsed.get() >= self.unlock_frame.get()
        {
            self.locked.set(false);
            let remaining = self.remaining_active_count();
            if remaining == 0 {
                self.finished.set(true);
                self.status_text.set(String::from("Every stick came free."));
                self.chance_text
                    .set(String::from("You cleared the tee-pee without a collapse."));
            } else {
                self.status_text.set(String::from("Clean pull."));
                self.chance_text
                    .set(next_chance_text(self.removed_count.get(), remaining));
            }
        }
    }

    pub fn remove_stick(&mut self, ctx: &NodeContext, _args: Event<Click>) {
        if self.locked.get() || self.finished.get() {
            return;
        }

        let Some(index) = clicked_repeat_index(ctx) else {
            return;
        };

        let current_sticks = self.sticks.get();
        if index >= current_sticks.len() || !current_sticks[index].active {
            return;
        }

        let removal_target = removal_layout(&current_sticks, index);
        let removed_total = self.removed_count.get() + 1;
        let remaining_after = current_sticks
            .iter()
            .enumerate()
            .filter(|(i, stick)| *i != index && stick.active)
            .count();

        self.locked.set(true);
        self.removed_count.set(removed_total);
        self.progress_text.set(progress_text(removed_total));
        self.sticks
            .ease_to(removal_target.clone(), REMOVAL_FRAMES, EasingCurve::OutBack);

        if remaining_after == 0 {
            self.finished.set(true);
            self.collapsed.set(false);
            self.status_text.set(String::from("Last stick."));
            self.chance_text.set(String::from(
                "The little tee-pee vanished in a clean final pull.",
            ));
            return;
        }

        let threshold = collapse_threshold(removed_total);
        let roll = self.next_random_unit();
        let roll_text = format!(
            "Collapse chance {}% / roll {}%",
            percent_label(threshold),
            percent_label(roll),
        );

        if roll < threshold {
            let collapse_target = collapse_layout(&removal_target, self.seed.get());
            self.sticks
                .ease_to_later(collapse_target, COLLAPSE_FRAMES, EasingCurve::InOutQuad);
            self.finished.set(true);
            self.collapsed.set(true);
            self.status_text.set(String::from("The tee-pee collapsed."));
            self.chance_text
                .set(format!("{roll_text}. Rebuild to stack another round."));
        } else {
            self.collapsed.set(false);
            self.status_text.set(String::from("Stick pulled."));
            self.chance_text.set(roll_text);
            self.unlock_frame
                .set(ctx.frames_elapsed.get() + REMOVAL_FRAMES + LOCK_BUFFER_FRAMES);
        }
    }

    pub fn reset_game(&mut self, ctx: &NodeContext, _args: Event<Click>) {
        let taps = self.rebuild_taps.get() + 1;
        self.rebuild_taps.set(taps);
        self.rebuild_pulse.set(1.0);
        self.rebuild_caption.set(format!("tap #{taps}"));
        self.reseed(ctx);
        self.reset_state();
        self.status_text.set(String::from("Fresh stack."));
        self.chance_text
            .set(format!("Rebuild button tapped {taps} time(s)."));
    }

    fn reset_state(&mut self) {
        self.sticks.set(initial_sticks());
        self.locked.set(false);
        self.finished.set(false);
        self.collapsed.set(false);
        self.removed_count.set(0);
        self.unlock_frame.set(0);
        self.status_text.set(String::from("Lift one bright stick."));
        self.chance_text.set(next_chance_text(0, STICK_COUNT));
        self.progress_text.set(progress_text(0));
    }

    fn reseed(&mut self, ctx: &NodeContext) {
        let elapsed = ctx.elapsed_time_millis() as u64;
        let mixed = advance_seed(elapsed ^ 0x9e37_79b9_7f4a_7c15 ^ self.seed.get().rotate_left(17));
        self.seed.set(mixed);
    }

    fn next_random_unit(&mut self) -> f64 {
        let next = advance_seed(self.seed.get());
        self.seed.set(next);
        next as f64 / u64::MAX as f64
    }

    fn remaining_active_count(&self) -> usize {
        self.sticks
            .get()
            .iter()
            .filter(|stick| stick.active)
            .count()
    }
}

fn progress_text(removed_count: usize) -> String {
    format!("Pulled {removed_count} / {STICK_COUNT}")
}

fn next_chance_text(removed_count: usize, remaining_count: usize) -> String {
    match remaining_count {
        0 => String::from("Every bright stick is gone."),
        1 => String::from("One stick remains. Pull it and the tee-pee is done."),
        _ => format!(
            "Next collapse chance {}%.",
            percent_label(collapse_threshold(removed_count + 1))
        ),
    }
}

fn collapse_threshold(removed_count: usize) -> f64 {
    (0.18 + removed_count.saturating_sub(1) as f64 * 0.07).clamp(0.18, 0.74)
}

fn percent_label(value_0_to_1: f64) -> u64 {
    (value_0_to_1.clamp(0.0, 0.999) * 100.0).round() as u64
}

fn advance_seed(seed: u64) -> u64 {
    seed.wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407)
}

fn centered_jitter(seed: &mut u64, amplitude: f64) -> f64 {
    *seed = advance_seed(*seed);
    let unit = *seed as f64 / u64::MAX as f64;
    (unit * 2.0 - 1.0) * amplitude
}

fn initial_sticks() -> Vec<Stick> {
    let palette = [
        "ff5f87", "ff8f3f", "ffd34f", "7eff70", "34f5cc", "43c7ff", "6a89ff", "8e6dff", "d85cff",
        "ff4fd8", "ff7f66",
    ];
    let segments = [
        ((324.0, 58.0), (126.0, 362.0)),
        ((346.0, 44.0), (170.0, 370.0)),
        ((372.0, 28.0), (214.0, 359.0)),
        ((402.0, 18.0), (266.0, 372.0)),
        ((434.0, 12.0), (324.0, 361.0)),
        ((466.0, 12.0), (382.0, 370.0)),
        ((494.0, 20.0), (440.0, 360.0)),
        ((520.0, 34.0), (496.0, 368.0)),
        ((542.0, 50.0), (556.0, 356.0)),
        ((404.0, 26.0), (606.0, 364.0)),
        ((448.0, 34.0), (652.0, 350.0)),
    ];

    segments
        .iter()
        .enumerate()
        .map(|(index, (start, end))| {
            stick_from_segment(
                *start,
                *end,
                Color::from_hex(palette[index]),
                12.0 + (index % 3) as f64,
            )
        })
        .collect()
}

fn stick_from_segment(start: (f64, f64), end: (f64, f64), color: Color, thickness: f64) -> Stick {
    let dx = end.0 - start.0;
    let dy = end.1 - start.1;
    let highlight_stroke = stroke(
        Color::rgba(255.into(), 248.into(), 234.into(), 90.into()),
        thickness * 0.18,
    );
    let main_stroke = stroke(color.clone(), thickness);
    let shadow_stroke = stroke(
        Color::rgba(6.into(), 8.into(), 16.into(), 70.into()),
        thickness + 6.0,
    );

    Stick {
        x: (start.0 + end.0) * 0.5,
        y: (start.1 + end.1) * 0.5,
        length: dx.hypot(dy),
        rotation: dy.atan2(dx).to_degrees(),
        opacity: 1.0,
        thickness,
        scale: 1.0,
        color,
        highlight_stroke,
        main_stroke,
        shadow_stroke,
        active: true,
    }
}

fn removal_layout(sticks: &[Stick], clicked_index: usize) -> Vec<Stick> {
    let mut target = sticks.to_vec();
    let clicked = &sticks[clicked_index];
    let side = if clicked.x <= PLAYFIELD_WIDTH * 0.5 {
        -1.0
    } else {
        1.0
    };

    target[clicked_index] = Stick {
        x: clicked.x + side * 210.0,
        y: clicked.y - 124.0,
        length: clicked.length,
        rotation: clicked.rotation + side * 28.0,
        opacity: 0.0,
        thickness: clicked.thickness,
        scale: 0.84,
        color: clicked.color.clone(),
        highlight_stroke: clicked.highlight_stroke.clone(),
        main_stroke: clicked.main_stroke.clone(),
        shadow_stroke: clicked.shadow_stroke.clone(),
        active: false,
    };

    target
}

fn collapse_layout(sticks: &[Stick], seed: u64) -> Vec<Stick> {
    let mut target = sticks.to_vec();
    let mut jitter_seed = seed;
    let active_indices: Vec<usize> = sticks
        .iter()
        .enumerate()
        .filter_map(|(index, stick)| stick.active.then_some(index))
        .collect();

    let count = active_indices.len().max(1);
    for (order, index) in active_indices.iter().enumerate() {
        let fan = if count == 1 {
            0.0
        } else {
            order as f64 / (count - 1) as f64 - 0.5
        };
        let x = PLAYFIELD_WIDTH * 0.5 + fan * 250.0 + centered_jitter(&mut jitter_seed, 18.0);
        let y = PLAYFIELD_HEIGHT * 0.79
            + (order % 3) as f64 * 10.0
            + centered_jitter(&mut jitter_seed, 8.0);
        let rotation = fan * 110.0 + centered_jitter(&mut jitter_seed, 18.0);

        target[*index] = Stick {
            x,
            y,
            length: sticks[*index].length,
            rotation,
            opacity: 1.0,
            thickness: sticks[*index].thickness,
            scale: 1.0,
            color: sticks[*index].color.clone(),
            highlight_stroke: sticks[*index].highlight_stroke.clone(),
            main_stroke: sticks[*index].main_stroke.clone(),
            shadow_stroke: sticks[*index].shadow_stroke.clone(),
            active: false,
        };
    }

    target
}

fn stroke(color: Color, width: f64) -> Stroke {
    Stroke {
        color: Property::new(color),
        width: Property::new(Size::Pixels(width.into())),
        cap: Property::new(StrokeCap::Round),
    }
}

fn clicked_repeat_index(ctx: &NodeContext) -> Option<usize> {
    ctx.slot_index.get().or_else(|| {
        ctx.local_stack_frame
            .resolve_symbol("i")
            .and_then(|variable| Numeric::try_coerce(variable.get_as_pax_value()).ok())
            .map(|numeric| numeric.to_float().round() as usize)
    })
}
