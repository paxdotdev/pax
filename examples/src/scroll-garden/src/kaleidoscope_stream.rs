use pax_kit::*;

use crate::kaleidoscope::{Kaleidoscope, KaleidoscopeSide, KaleidoscopeVariant};

#[pax]
#[custom(Default)]
#[file("kaleidoscope_stream.pax")]
pub struct KaleidoscopeStream {
    pub side: Property<KaleidoscopeSide>,
    pub variant: Property<KaleidoscopeVariant>,
    pub count: Property<usize>,
    /// Values <= 4 are interpreted as multiples of computed tendril size.
    pub spacing: Property<f64>,
    /// Values <= 1 are interpreted as a fraction of the stream width.
    pub size: Property<f64>,
    pub playhead: Property<f64>,

    pub _items: Property<Vec<KaleidoscopeStreamItem>>,
}

impl Default for KaleidoscopeStream {
    fn default() -> Self {
        Self {
            side: Property::new(KaleidoscopeSide::Left),
            variant: Property::new(KaleidoscopeVariant::Crimson),
            count: Property::new(2),
            spacing: Property::new(0.9),
            size: Property::new(0.4),
            playhead: Property::new(0.0),
            _items: Property::new(vec![]),
        }
    }
}

#[pax]
#[engine_import_path("pax_engine")]
pub struct KaleidoscopeStreamItem {
    pub y: f64,
    pub size: f64,
    pub layers: usize,
    pub offset: f64,
}

impl KaleidoscopeStream {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        let bounds = ctx.bounds_self.clone();
        let count = self.count.clone();
        let spacing = self.spacing.clone();
        let size = self.size.clone();
        let deps = [
            bounds.untyped(),
            count.untyped(),
            spacing.untyped(),
            size.untyped(),
        ];
        self._items.replace_with(Property::computed(
            move || {
                let count_val = count.get().max(1);
                let (width, _height) = bounds.get();
                let raw_size = size.get();
                let base_size = if raw_size <= 1.0 {
                    (width * raw_size).max(120.0)
                } else {
                    raw_size.max(120.0)
                };
                let raw_spacing = spacing.get();
                let spacing_val = if raw_spacing <= 4.0 {
                    (base_size * raw_spacing).max(120.0)
                } else {
                    raw_spacing.max(120.0)
                };
                (0..count_val)
                    .map(|i| {
                        let idx = i as f64;
                        let y = (idx - 1.0) * spacing_val;
                        let size_scale = (1.0 - idx * 0.12).max(0.7);
                        let layers = 1;
                        let offset = idx * 120.0;
                        KaleidoscopeStreamItem {
                            y,
                            size: base_size * size_scale,
                            layers,
                            offset,
                        }
                    })
                    .collect()
            },
            &deps,
        ));
    }
}
