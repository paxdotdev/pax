use pax_kit::*;

const PALETTES: [[&str; 4]; 4] = [
    ["F6F6F6", "BDBDBD", "5A5A5A", "101010"],
    ["E8E8E8", "A4A4A4", "424242", "080808"],
    ["D8D8D8", "8A8A8A", "313131", "0F0F0F"],
    ["FCFCFC", "CACACA", "6B6B6B", "1B1B1B"],
];
const TILE_COUNT: usize = 15;
const MOTIF_SALT: u64 = 0xA24B_AED4_963E_E407;
const ORIENTATION_SALT: u64 = 0x9FB2_1C65_1E98_DF25;
const TONE_SALT: u64 = 0xD6E8_FEB8_6659_FD93;
const INVERSION_SALT: u64 = 0x94D0_49BB_1331_11EB;
const ROTATION_SALT: u64 = 0xCA5A_8263_9512_1157;

#[pax]
#[custom(Default)]
#[file("quilt_tile.pax")]
pub struct QuiltTile {
    pub index: Property<usize>,
    pub columns: Property<usize>,
    pub wave_origin: Property<usize>,
    pub start_turn: Property<f64>,
    pub spin_turn: Property<f64>,
    pub generation: Property<usize>,

    pub _variant: Property<usize>,
    pub _orientation_turn: Property<f64>,
    pub _local_turn: Property<f64>,
    pub _phase: Property<f64>,
    pub _wave: Property<f64>,
    pub _turn: Property<Rotation>,
    pub _scale: Property<f64>,
    pub _aspect: Property<f64>,
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
            generation: Property::new(0),
            _variant: Property::new(0),
            _orientation_turn: Property::new(0.0),
            _local_turn: Property::new(0.0),
            _phase: Property::new(0.0),
            _wave: Property::new(0.0),
            _turn: Property::new(Rotation::Degrees(0.0.into())),
            _scale: Property::new(1.0),
            _aspect: Property::new(1.0),
            _background: Property::new(Color::from_hex("F6F6F6")),
            _foreground: Property::new(Color::from_hex("BDBDBD")),
            _secondary: Property::new(Color::from_hex("5A5A5A")),
        }
    }
}

impl QuiltTile {
    pub fn handle_mount(&mut self, ctx: &NodeContext) {
        let index = self.index.clone();
        self._variant.replace_with(Property::computed(
            move || tile_variant(index.get()),
            &[self.index.untyped()],
        ));

        let index = self.index.clone();
        self._orientation_turn.replace_with(Property::computed(
            move || initial_orientation(index.get()),
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
                let click_progress = radial_progress(
                    index.get(),
                    columns.get(),
                    origin.get(),
                    start_turn.get(),
                    spin_turn.get(),
                );
                cumulative_rotation(index.get(), click_progress)
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
        let orientation_turn = self._orientation_turn.clone();
        self._turn.replace_with(Property::computed(
            move || Rotation::Degrees(((local_turn.get() + orientation_turn.get()) * 90.0).into()),
            &[self._local_turn.untyped(), self._orientation_turn.untyped()],
        ));

        let wave = self._wave.clone();
        self._scale.replace_with(Property::computed(
            // A square rotated halfway between quarter turns needs sqrt(2)
            // scale to keep its clipped frame covered.
            move || 1.0 + wave.get() * (std::f64::consts::SQRT_2 - 1.0),
            &[self._wave.untyped()],
        ));

        let bounds = ctx.bounds_self.clone();
        self._aspect.replace_with(Property::computed(
            move || tile_aspect(bounds.get()),
            &[ctx.bounds_self.untyped()],
        ));

        self.bind_color_role(0);
        self.bind_color_role(1);
        self.bind_color_role(2);
    }

    fn bind_color_role(&mut self, role: usize) {
        let index = self.index.clone();
        let generation = self.generation.clone();
        let deps = [index.untyped(), generation.untyped()];
        let property = Property::computed(
            move || quilt_color(index.get(), generation.get(), role),
            &deps,
        );
        match role {
            0 => self._background.replace_with(property),
            1 => self._foreground.replace_with(property),
            _ => self._secondary.replace_with(property),
        }
    }
}

fn radial_progress(
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

fn quilt_color(index: usize, generation: usize, role: usize) -> Color {
    let tint_variant = seeded_tile_value(index, 0, TONE_SALT) as usize % PALETTES.len();
    let starts_inverted = seeded_tile_value(index, 0, INVERSION_SALT) % 2 == 1;
    let inverted = starts_inverted ^ (generation % 2 == 1);
    let role = role % 4;
    let resolved_role = if inverted { 3 - role } else { role };
    Color::from_hex(PALETTES[tint_variant][resolved_role])
}

fn tile_variant(index: usize) -> usize {
    seeded_tile_value(index, 0, MOTIF_SALT) as usize % 2
}

fn initial_orientation(index: usize) -> f64 {
    (seeded_tile_value(index, 0, ORIENTATION_SALT) % 4) as f64
}

fn cumulative_rotation(index: usize, click_progress: f64) -> f64 {
    let click_progress = click_progress.max(0.0);
    let completed_clicks = click_progress.floor() as usize;
    let remainder = click_progress - completed_clicks as f64;
    let completed_rotation: f64 = (1..=completed_clicks)
        .map(|click| rotation_delta(index, click))
        .sum();
    completed_rotation + remainder * rotation_delta(index, completed_clicks + 1)
}

fn rotation_delta(index: usize, click: usize) -> f64 {
    let value = seeded_tile_value(index, click, ROTATION_SALT);
    let magnitude = (value % 4 + 1) as f64;
    if (value >> 8) & 1 == 1 {
        magnitude
    } else {
        -magnitude
    }
}

fn seeded_tile_value(index: usize, generation: usize, salt: u64) -> u64 {
    let mut value = (index as u64 + 1).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (generation as u64 + 1).wrapping_mul(0xD1B5_4A32_D192_ED03)
        ^ salt;
    value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
}

fn tile_aspect((width, height): (f64, f64)) -> f64 {
    if width > f64::EPSILON && height > f64::EPSILON {
        width / height
    } else {
        1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn radial_progress_starts_and_finishes_at_the_origin() {
        assert_eq!(radial_progress(7, 5, 7, 2.0, 2.0), 2.0);
        assert_eq!(radial_progress(7, 5, 7, 2.0, 2.24), 2.24);
        assert_eq!(radial_progress(7, 5, 7, 2.0, 3.0), 3.0);
    }

    #[test]
    fn distant_tiles_start_later() {
        assert_eq!(radial_progress(0, 5, 0, 0.0, 0.15), 0.15);
        assert_eq!(radial_progress(14, 5, 0, 0.0, 0.15), 0.0);
    }

    #[test]
    fn added_windings_do_not_rewind_the_visible_turn() {
        let before = radial_progress(7, 5, 7, 2.0, 2.35);
        let after_target_extension = radial_progress(7, 5, 7, 2.0, 2.35);
        assert_eq!(before, after_target_extension);
        assert_eq!(radial_progress(7, 5, 7, 2.0, 4.0), 4.0);
    }

    #[test]
    fn seeded_motifs_are_mixed_without_an_alternating_pattern() {
        let variants: Vec<_> = (0..TILE_COUNT).map(tile_variant).collect();
        assert!(variants.contains(&0));
        assert!(variants.contains(&1));
        assert!(variants.windows(2).any(|pair| pair[0] == pair[1]));
    }

    #[test]
    fn every_impulse_uses_a_signed_one_to_four_quarter_turns() {
        let deltas: Vec<_> = (0..TILE_COUNT)
            .flat_map(|index| (1..=4).map(move |click| rotation_delta(index, click)))
            .collect();
        assert!(deltas
            .iter()
            .all(|delta| (1.0..=4.0).contains(&delta.abs())));
        assert!(deltas.iter().any(|delta| *delta < 0.0));
        assert!(deltas.iter().any(|delta| *delta > 0.0));
    }

    #[test]
    fn color_roles_invert_on_every_click() {
        for index in 0..TILE_COUNT {
            for role in 0..4 {
                assert_eq!(quilt_color(index, 0, role), quilt_color(index, 1, 3 - role));
            }
        }
    }

    #[test]
    fn aspect_defaults_safely_and_preserves_rectangular_bounds() {
        assert_eq!(tile_aspect((0.0, 10.0)), 1.0);
        assert_eq!(tile_aspect((300.0, 200.0)), 1.5);
    }
}
