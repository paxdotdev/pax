use pax_kit::*;

use crate::kaleidoscope::KaleidoscopeVariant;

#[pax]
#[custom(Default)]
#[file("tendril.pax")]
pub struct Tendril {
    pub playhead: Property<f64>,
    pub seed: Property<f64>,
    pub variant: Property<KaleidoscopeVariant>,
    pub opacity: Property<f64>,

    pub _segment: Property<f64>,
    pub _thickness: Property<f64>,
    pub _gem: Property<f64>,
    pub _angle_0: Property<f64>,
    pub _angle_1: Property<f64>,
    pub _angle_2: Property<f64>,
    pub _angle_3: Property<f64>,
    pub _shape_0: Property<usize>,
    pub _shape_1: Property<usize>,
    pub _shape_2: Property<usize>,
    pub _shape_3: Property<usize>,
}

impl Default for Tendril {
    fn default() -> Self {
        Self {
            playhead: Property::new(0.0),
            seed: Property::new(0.0),
            variant: Property::new(KaleidoscopeVariant::Crimson),
            opacity: Property::new(1.0),
            _segment: Property::new(120.0),
            _thickness: Property::new(36.0),
            _gem: Property::new(22.0),
            _angle_0: Property::new(0.0),
            _angle_1: Property::new(0.0),
            _angle_2: Property::new(0.0),
            _angle_3: Property::new(0.0),
            _shape_0: Property::new(0),
            _shape_1: Property::new(0),
            _shape_2: Property::new(0),
            _shape_3: Property::new(0),
        }
    }
}

impl Tendril {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        let bounds = ctx.bounds_self.clone();
        let deps = [bounds.untyped()];
        self._segment.replace_with(Property::computed(
            move || {
                let (width, _) = bounds.get();
                let base = if width <= 1.0 { 160.0 } else { width };
                ((base * 0.28).max(90.0)) * 3.0
            },
            &deps,
        ));

        let segment = self._segment.clone();
        let deps = [segment.untyped()];
        self._thickness.replace_with(Property::computed(
            move || (segment.get() * 0.38).max(20.0),
            &deps,
        ));

        let segment = self._segment.clone();
        let deps = [segment.untyped()];
        self._gem.replace_with(Property::computed(
            move || (segment.get() * 0.18).max(18.0),
            &deps,
        ));

        let playhead = self.playhead.clone();
        let seed = self.seed.clone();
        let deps = [playhead.untyped(), seed.untyped()];
        self._angle_0.replace_with(Property::computed(
            move || joint_angle(playhead.get(), seed.get(), 0),
            &deps,
        ));

        let playhead = self.playhead.clone();
        let seed = self.seed.clone();
        let deps = [playhead.untyped(), seed.untyped()];
        self._angle_1.replace_with(Property::computed(
            move || joint_angle(playhead.get(), seed.get(), 1),
            &deps,
        ));

        let playhead = self.playhead.clone();
        let seed = self.seed.clone();
        let deps = [playhead.untyped(), seed.untyped()];
        self._angle_2.replace_with(Property::computed(
            move || joint_angle(playhead.get(), seed.get(), 2),
            &deps,
        ));

        let playhead = self.playhead.clone();
        let seed = self.seed.clone();
        let deps = [playhead.untyped(), seed.untyped()];
        self._angle_3.replace_with(Property::computed(
            move || joint_angle(playhead.get(), seed.get(), 3),
            &deps,
        ));

        let seed = self.seed.clone();
        let deps = [seed.untyped()];
        self._shape_0.replace_with(Property::computed(
            move || shape_index(seed.get(), 0),
            &deps,
        ));

        let seed = self.seed.clone();
        let deps = [seed.untyped()];
        self._shape_1.replace_with(Property::computed(
            move || shape_index(seed.get(), 1),
            &deps,
        ));

        let seed = self.seed.clone();
        let deps = [seed.untyped()];
        self._shape_2.replace_with(Property::computed(
            move || shape_index(seed.get(), 2),
            &deps,
        ));

        let seed = self.seed.clone();
        let deps = [seed.untyped()];
        self._shape_3.replace_with(Property::computed(
            move || shape_index(seed.get(), 3),
            &deps,
        ));
    }
}

fn joint_angle(playhead: f64, seed: f64, index: i32) -> f64 {
    let t = (playhead / 1000.0).clamp(0.0, 1.0) * std::f64::consts::PI * 2.0;
    let phase = seed * 0.08 + index as f64 * 0.6;
    let coil = (t * 0.85 + phase).sin() * 0.6 + 0.4;
    let amplitude = (18.0 + coil * 14.0) * 4.0;
    let bias = (t * 0.45 + seed * 0.12).cos() * 6.0;
    let decay = 1.0 - index as f64 * 0.12;
    ((t + phase).sin() * amplitude + bias) * decay
}

fn shape_index(seed: f64, index: i32) -> usize {
    // Stable pseudo-randomness keeps each joint visually varied without changing
    // the authored scene graph while the user scrubs the timeline.
    let value = (seed * 0.31 + index as f64 * 1.73).sin() * 0.5 + 0.5;
    (value * 11.999).floor().clamp(0.0, 11.0) as usize
}
