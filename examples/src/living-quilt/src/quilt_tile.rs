use pax_kit::*;

const PALETTES: [[&str; 5]; 3] = [
    ["123DCC", "F12E7A", "FF6A52", "F7C928", "9B70DB"],
    ["006FC5", "D633A4", "F0784E", "E4D43A", "795DCF"],
    ["2448B8", "E74891", "F25A48", "F2B82B", "AB74D4"],
];
const TILE_COUNT: usize = 15;

#[pax]
#[custom(Default)]
#[file("quilt_tile.pax")]
pub struct QuiltTile {
    pub index: Property<usize>,
    pub columns: Property<usize>,
    pub wave_origin: Property<usize>,
    pub start_turn: Property<f64>,
    pub spin_turn: Property<f64>,
    pub palette: Property<usize>,

    pub _motif: Property<usize>,
    pub _flip: Property<bool>,
    pub _alternate: Property<bool>,
    pub _local_turn: Property<f64>,
    pub _phase: Property<f64>,
    pub _wave: Property<f64>,
    pub _turn: Property<Rotation>,
    pub _scale: Property<f64>,
    pub _shift_x: Property<f64>,
    pub _shift_y: Property<f64>,
    pub _background: Property<Color>,
    pub _foreground: Property<Color>,
    pub _secondary: Property<Color>,
}

impl Default for QuiltTile {
    fn default() -> Self {
        Self {
            index: Property::new(0),
            columns: Property::new(5),
            wave_origin: Property::new(0),
            start_turn: Property::new(0.0),
            spin_turn: Property::new(0.0),
            palette: Property::new(0),
            _motif: Property::new(0),
            _flip: Property::new(false),
            _alternate: Property::new(false),
            _local_turn: Property::new(0.0),
            _phase: Property::new(0.0),
            _wave: Property::new(0.0),
            _turn: Property::new(Rotation::Degrees(0.0.into())),
            _scale: Property::new(1.0),
            _shift_x: Property::new(0.0),
            _shift_y: Property::new(0.0),
            _background: Property::new(Color::from_hex("F7F1E5")),
            _foreground: Property::new(Color::from_hex("11110F")),
            _secondary: Property::new(Color::from_hex("256FBE")),
        }
    }
}

impl QuiltTile {
    pub fn handle_mount(&mut self, _ctx: &NodeContext) {
        let index = self.index.clone();
        self._motif.replace_with(Property::computed(
            move || index.get() % 4,
            &[self.index.untyped()],
        ));

        let index = self.index.clone();
        self._flip.replace_with(Property::computed(
            move || index.get() % 8 < 4,
            &[self.index.untyped()],
        ));

        let index = self.index.clone();
        self._alternate.replace_with(Property::computed(
            move || index.get() % 3 == 0,
            &[self.index.untyped()],
        ));

        let index = self.index.clone();
        let columns = self.columns.clone();
        let origin = self.wave_origin.clone();
        let start_turn = self.start_turn.clone();
        let spin_turn = self.spin_turn.clone();
        let turn_deps = [
            index.untyped(),
            columns.untyped(),
            origin.untyped(),
            start_turn.untyped(),
            spin_turn.untyped(),
        ];
        self._local_turn.replace_with(Property::computed(
            move || {
                radial_turn(
                    index.get(),
                    columns.get(),
                    origin.get(),
                    start_turn.get(),
                    spin_turn.get(),
                )
            },
            &turn_deps,
        ));

        let local_turn = self._local_turn.clone();
        self._phase.replace_with(Property::computed(
            move || local_turn.get().rem_euclid(1.0),
            &[self._local_turn.untyped()],
        ));

        let phase = self._phase.clone();
        self._wave.replace_with(Property::computed(
            move || (phase.get() * std::f64::consts::PI).sin().powf(0.82),
            &[self._phase.untyped()],
        ));

        let local_turn = self._local_turn.clone();
        self._turn.replace_with(Property::computed(
            move || Rotation::Degrees((local_turn.get() * 90.0).into()),
            &[self._local_turn.untyped()],
        ));

        let wave = self._wave.clone();
        self._scale.replace_with(Property::computed(
            move || 1.0 + wave.get() * 0.415,
            &[self._wave.untyped()],
        ));

        let wave = self._wave.clone();
        let index = self.index.clone();
        self._shift_x.replace_with(Property::computed(
            move || {
                let direction = if index.get() % 3 == 0 { -1.0 } else { 1.0 };
                wave.get() * direction * 1.5
            },
            &[self._wave.untyped(), self.index.untyped()],
        ));

        let wave = self._wave.clone();
        let index = self.index.clone();
        self._shift_y.replace_with(Property::computed(
            move || {
                let direction = if index.get() % 4 < 2 { -1.0 } else { 1.0 };
                wave.get() * direction * 1.5
            },
            &[self._wave.untyped(), self.index.untyped()],
        ));

        self.bind_color_role(0);
        self.bind_color_role(1);
        self.bind_color_role(2);
    }

    fn bind_color_role(&mut self, role: usize) {
        let index = self.index.clone();
        let palette = self.palette.clone();
        let deps = [index.untyped(), palette.untyped()];
        let property =
            Property::computed(move || quilt_color(index.get(), palette.get(), role), &deps);
        match role {
            0 => self._background.replace_with(property),
            1 => self._foreground.replace_with(property),
            _ => self._secondary.replace_with(property),
        }
    }
}

fn radial_turn(
    index: usize,
    columns: usize,
    origin: usize,
    start_turn: f64,
    spin_turn: f64,
) -> f64 {
    let columns = columns.max(1);
    let rows = TILE_COUNT.div_ceil(columns);
    let row = index / columns;
    let column = index % columns;
    let origin_row = origin / columns;
    let origin_column = origin % columns;
    let distance = (row.abs_diff(origin_row) as f64).hypot(column.abs_diff(origin_column) as f64);
    let farthest_row = origin_row.max(rows.saturating_sub(1).saturating_sub(origin_row));
    let farthest_column =
        origin_column.max(columns.saturating_sub(1).saturating_sub(origin_column));
    let max_distance = (farthest_row as f64).hypot(farthest_column as f64).max(1.0);
    let delay = distance / max_distance * 0.48;
    let burst_delta = (spin_turn - start_turn).max(0.0);
    let local_delta = if burst_delta <= 1.0 {
        ((burst_delta - delay) / (1.0 - delay)).clamp(0.0, 1.0)
    } else {
        burst_delta
    };
    start_turn + local_delta
}

fn quilt_color(index: usize, palette: usize, role: usize) -> Color {
    let colors = PALETTES[palette % PALETTES.len()];
    let paper = "F7F1E5";
    let ink = "11110F";
    let color = colors[(index + palette) % colors.len()];
    let next = colors[(index + palette + 2) % colors.len()];
    let background = match index % 8 {
        0 | 7 => paper,
        2 => ink,
        1 => colors[0],
        3 => colors[1],
        4 => colors[4],
        5 => colors[3],
        _ => colors[2],
    };

    let accent = if next == background {
        colors[(index + palette + 3) % colors.len()]
    } else {
        next
    };
    let hex = match role {
        0 => background,
        1 if background == paper => ink,
        1 if background == ink => paper,
        1 if index % 3 == 0 => ink,
        1 => paper,
        _ if background == ink => color,
        _ => accent,
    };
    Color::from_hex(hex)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn radial_turn_starts_and_finishes_at_the_origin() {
        assert_eq!(radial_turn(7, 5, 7, 2.0, 2.0), 2.0);
        assert_eq!(radial_turn(7, 5, 7, 2.0, 2.24), 2.24);
        assert_eq!(radial_turn(7, 5, 7, 2.0, 3.0), 3.0);
    }

    #[test]
    fn distant_tiles_start_later() {
        assert_eq!(radial_turn(0, 5, 0, 0.0, 0.15), 0.15);
        assert_eq!(radial_turn(14, 5, 0, 0.0, 0.15), 0.0);
    }

    #[test]
    fn added_windings_do_not_rewind_the_visible_turn() {
        let before = radial_turn(7, 5, 7, 2.0, 2.35);
        let after_target_extension = radial_turn(7, 5, 7, 2.0, 2.35);
        assert_eq!(before, after_target_extension);
        assert_eq!(radial_turn(7, 5, 7, 2.0, 4.0), 4.0);
    }
}
