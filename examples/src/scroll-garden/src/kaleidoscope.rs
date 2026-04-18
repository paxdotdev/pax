use pax_kit::*;

use crate::tendril::Tendril;

const VARIANT_PHASE: [f64; 5] = [0.0, 0.18, 0.36, 0.54, 0.72];

#[pax]
#[custom(Default)]
#[file("kaleidoscope.pax")]
pub struct Kaleidoscope {
    pub side: Property<KaleidoscopeSide>,
    pub layers: Property<usize>,
    pub variant: Property<KaleidoscopeVariant>,
    pub playhead: Property<f64>,
    pub enable_tendrils: Property<bool>,

    pub _angles: Property<Vec<f64>>,
    pub _layers: Property<Vec<KaleidoscopeLayer>>,
    pub _phase: Property<f64>,
    pub _fold: Property<f64>,
    pub _twist: Property<Rotation>,
    pub _drift: Property<f64>,
    pub _growth: Property<f64>,
    pub _coil: Property<f64>,
    pub _side_factor: Property<f64>,
}

impl Default for Kaleidoscope {
    fn default() -> Self {
        Self {
            side: Property::new(KaleidoscopeSide::Left),
            layers: Property::new(3),
            variant: Property::new(KaleidoscopeVariant::Crimson),
            playhead: Property::new(0.0),
            // Disabled by default until viewport tile culling makes these dense
            // decorative subtrees cheap enough for the Apple scroller fixture.
            enable_tendrils: Property::new(false),
            _angles: Property::new(vec![]),
            _layers: Property::new(vec![]),
            _phase: Property::new(0.0),
            _fold: Property::new(0.8),
            _twist: Property::new(Rotation::Degrees(Numeric::F64(0.0))),
            _drift: Property::new(0.0),
            _growth: Property::new(1.0),
            _coil: Property::new(1.0),
            _side_factor: Property::new(1.0),
        }
    }
}

#[pax]
#[engine_import_path("pax_engine")]
pub enum KaleidoscopeSide {
    #[default]
    Left,
    Right,
}

#[pax]
#[engine_import_path("pax_engine")]
pub enum KaleidoscopeVariant {
    #[default]
    Crimson,
    Nocturne,
    Lagoon,
    Fern,
    Violet,
}

#[pax]
#[engine_import_path("pax_engine")]
pub struct KaleidoscopeLayer {
    pub index: usize,
    pub scale: f64,
    pub opacity: f64,
    pub rotation_deg: f64,
    pub drift: f64,
}

impl Kaleidoscope {
    pub fn on_mount(&mut self, _ctx: &NodeContext) {
        self._angles.replace_with(Property::computed(
            move || (0..6).map(|i| i as f64 * 60.0).collect(),
            &[],
        ));

        let layers = self.layers.clone();
        let deps = [layers.untyped()];
        self._layers.replace_with(Property::computed(
            move || {
                let count = layers.get().max(1);
                (0..count)
                    .map(|i| {
                        let scale = (1.0 - i as f64 * 0.18).max(0.46);
                        let opacity = (0.7 - i as f64 * 0.12).max(0.2);
                        let rotation_deg = i as f64 * 8.0;
                        let drift = i as f64 * 18.0;
                        KaleidoscopeLayer {
                            index: i,
                            scale,
                            opacity,
                            rotation_deg,
                            drift,
                        }
                    })
                    .collect()
            },
            &deps,
        ));

        let playhead = self.playhead.clone();
        let variant = self.variant.clone();
        let deps = [playhead.untyped(), variant.untyped()];
        self._phase.replace_with(Property::computed(
            move || {
                let base = (playhead.get() / 1000.0).clamp(0.0, 1.0);
                let variant_offset = match variant.get() {
                    KaleidoscopeVariant::Crimson => VARIANT_PHASE[0],
                    KaleidoscopeVariant::Nocturne => VARIANT_PHASE[1],
                    KaleidoscopeVariant::Lagoon => VARIANT_PHASE[2],
                    KaleidoscopeVariant::Fern => VARIANT_PHASE[3],
                    KaleidoscopeVariant::Violet => VARIANT_PHASE[4],
                };
                (base + variant_offset) % 1.0
            },
            &deps,
        ));

        let phase = self._phase.clone();
        self._fold.replace_with(Property::computed(
            move || {
                let wave = (phase.get() * std::f64::consts::PI * 2.0).sin();
                (0.65 + wave * 0.25).clamp(0.35, 1.0)
            },
            &[self._phase.untyped()],
        ));

        let phase = self._phase.clone();
        self._twist.replace_with(Property::computed(
            move || {
                let wave = (phase.get() * std::f64::consts::PI * 2.0).cos();
                Rotation::Degrees(Numeric::F64(wave * 16.0))
            },
            &[self._phase.untyped()],
        ));

        let phase = self._phase.clone();
        self._drift.replace_with(Property::computed(
            move || (phase.get() * std::f64::consts::PI * 2.0).sin() * 24.0,
            &[self._phase.untyped()],
        ));

        let phase = self._phase.clone();
        self._coil.replace_with(Property::computed(
            move || {
                let wave = (phase.get() * std::f64::consts::PI * 2.0).cos();
                (0.55 + wave * 0.25).clamp(0.25, 0.95)
            },
            &[self._phase.untyped()],
        ));

        let playhead = self.playhead.clone();
        let phase = self._phase.clone();
        self._growth.replace_with(Property::computed(
            move || {
                let progress = (playhead.get() / 1000.0).clamp(0.0, 1.0);
                let pulse = (phase.get() * std::f64::consts::PI * 2.0).sin();
                let base = 0.6 + progress * 0.7;
                (base * (0.9 + pulse * 0.1)).clamp(0.5, 1.5)
            },
            &[self.playhead.untyped(), self._phase.untyped()],
        ));

        let side = self.side.clone();
        let deps = [side.untyped()];
        self._side_factor.replace_with(Property::computed(
            move || match side.get() {
                // Mirror the right-edge stream while keeping joint math shared.
                KaleidoscopeSide::Left => 1.0,
                KaleidoscopeSide::Right => -1.0,
            },
            &deps,
        ));
    }
}
