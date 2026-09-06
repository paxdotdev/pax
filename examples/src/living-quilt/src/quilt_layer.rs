use pax_kit::*;

#[pax]
#[custom(Default)]
#[file("quilt_layer.pax")]
pub struct QuiltLayer {
    pub is_compact: Property<bool>,
    pub start_turn: Property<f64>,
    pub generation: Property<usize>,
    pub wave_origin: Property<usize>,
    pub spin_turn: Property<f64>,
}

impl Default for QuiltLayer {
    fn default() -> Self {
        Self {
            is_compact: Property::new(false),
            start_turn: Property::new(0.0),
            generation: Property::new(0),
            wave_origin: Property::new(0),
            spin_turn: Property::new(0.0),
        }
    }
}
