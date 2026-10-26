//! Cell composition has no dependency on drawing or the scene tree.
use crate::font::{glyph, normalize_message};
use crate::ink::MAX_INK_COLUMNS;

pub const ROWS: usize = 29;
pub const MAX_COLUMNS: usize = 96;
pub const GLYPH_SCALE: usize = 3;
pub const LETTER_PITCH: usize = 6 * GLYPH_SCALE;
pub const SPACE_PITCH: usize = LETTER_PITCH * 2 / 3;

/// Logarithmic frequency control: the midpoint preserves the original wave spacing.
pub fn frequency_multiplier(value: f64) -> f64 {
    2f64.powf(value.clamp(0., 1.) * 4. - 2.)
}

/// Address the source message, including its blank wrap gap, independently of the viewport.
pub fn foreground_column(x: usize, scroll: usize, period: usize) -> usize {
    (x + scroll % period) % period
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Cell {
    #[default]
    Off,
    Cyan,
    Magenta,
    Yellow,
    Text,
}

/// A bounded row-major display. Cell occupancy is discrete; lamp shading is separate.
#[derive(Default)]
pub struct Display {
    pub columns: usize,
    pub cells: Vec<Cell>,
    foreground_columns: Vec<u32>,
}
impl Display {
    pub fn resize(&mut self, columns: usize) {
        self.columns = columns.clamp(1, MAX_COLUMNS);
        self.cells.resize(self.columns * ROWS, Cell::Off);
    }
    pub fn set_text(&mut self, text: &str) {
        // Cache variable-width text as columns so animation only visits visible cells.
        self.foreground_columns.clear();
        let top = (ROWS - 7 * GLYPH_SCALE) / 2;
        for ch in normalize_message(text).chars() {
            if ch == ' ' {
                self.foreground_columns.extend([0; SPACE_PITCH]);
                continue;
            }
            let rows = glyph(ch);
            for column in 0..5 {
                let mut bits = 0;
                for (row, value) in rows.iter().enumerate() {
                    if value & (1 << (4 - column)) != 0 {
                        bits |= ((1 << GLYPH_SCALE) - 1) << (top + row * GLYPH_SCALE);
                    }
                }
                self.foreground_columns.extend([bits; GLYPH_SCALE]);
            }
            self.foreground_columns.extend([0; GLYPH_SCALE]);
        }
    }
    pub fn set_drawing(&mut self, columns: &[u32]) {
        self.foreground_columns = columns
            .iter()
            .take(MAX_INK_COLUMNS)
            .map(|bits| bits & ((1 << ROWS) - 1))
            .collect();
    }
    pub fn period(&self) -> usize {
        self.foreground_columns.len() + 18
    }
    pub fn compose(
        &mut self,
        phase: f64,
        scroll: usize,
        amplitudes: [f64; 3],
        frequencies: [f64; 3],
    ) {
        self.cells.fill(Cell::Off);
        let center = (ROWS - 1) as f64 * 0.5;
        for (index, color) in [Cell::Yellow, Cell::Magenta, Cell::Cyan]
            .into_iter()
            .enumerate()
        {
            let channel = 2 - index;
            let mut previous = None;
            for x in 0..self.columns {
                let angle = x as f64
                    * [0.16, 0.22, 0.11][channel]
                    * frequency_multiplier(frequencies[channel])
                    + phase * [1.1, -0.8, 0.65][channel]
                    + channel as f64 * 2.1;
                let y = (center + angle.sin() * amplitudes[channel].clamp(0., 1.) * (center - 1.))
                    .round() as usize;
                // Fill vertical steps at this column so steep slopes remain connected,
                // without fractional coverage or secondary dim edge cells.
                let before: usize = previous.unwrap_or(y);
                for row in before.min(y)..=before.max(y) {
                    self.cells[row * self.columns + x] = color;
                }
                previous = Some(y);
            }
        }
        let period = self.period();
        for x in 0..self.columns {
            let source = foreground_column(x, scroll, period);
            if let Some(&bits) = self.foreground_columns.get(source) {
                let mut remaining = bits;
                while remaining != 0 {
                    let row = remaining.trailing_zeros() as usize;
                    self.cells[row * self.columns + x] = Cell::Text;
                    remaining &= remaining - 1;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn frequency_changes_wave_spacing_without_changing_other_channels() {
        assert_eq!(frequency_multiplier(0.), 0.25);
        assert_eq!(frequency_multiplier(0.5), 1.);
        assert_eq!(frequency_multiplier(1.), 4.);
        let mut d = Display::default();
        d.resize(96);
        d.compose(0., 0, [1., 0., 0.], [0.5; 3]);
        let original = d.cells.clone();
        let crossings = |cells: &[Cell]| {
            cells[14 * 96..15 * 96]
                .windows(2)
                .filter(|pair| pair[0] != Cell::Cyan && pair[1] == Cell::Cyan)
                .count()
        };
        let low = crossings(&original);
        d.compose(0., 0, [1., 0., 0.], [0.5, 0., 1.]);
        assert_eq!(d.cells, original);
        d.compose(0., 0, [1., 0., 0.], [1., 0.5, 0.5]);
        assert!(crossings(&d.cells) >= 3 * low);
    }

    #[test]
    fn foreground_addresses_follow_the_message_through_movement_and_wrap() {
        let mut d = Display::default();
        for drawing in [false, true] {
            if drawing {
                d.set_drawing(&vec![1 << 14; 300]);
            } else {
                d.set_text("DREAM IT. BUILD IT.");
            }
            let period = d.period();
            for scroll in [0, 57, period - 1, period + 20] {
                for x in 1..56 {
                    let address = foreground_column(x, scroll, period);
                    assert_eq!(address, foreground_column(x - 1, scroll + 1, period));
                    assert_eq!(address, foreground_column(x, scroll + period, period));
                }
            }
        }
    }

    #[test]
    fn drawing_overwrites_waves_and_scrolls_with_the_same_wrap() {
        let mut d = Display::default();
        d.resize(28);
        d.set_drawing(&[0, 1 << 14, u32::MAX]);
        d.compose(0., 0, [0.; 3], [0.5; 3]);
        assert_eq!(d.cells[14 * 28 + 1], Cell::Text);
        assert!((0..ROWS).all(|row| d.cells[row * 28 + 2] == Cell::Text));
        let expected = d.cells.clone();
        d.compose(0., d.period(), [0.; 3], [0.5; 3]);
        assert_eq!(d.cells, expected);
        d.compose(0., 1, [0.; 3], [0.5; 3]);
        assert_eq!(d.cells[14 * 28], Cell::Text);
        d.set_drawing(&vec![1 << 14; 300]);
        d.compose(0., 220, [0.; 3], [0.5; 3]);
        assert!(d.cells[14 * 28..15 * 28]
            .iter()
            .all(|cell| *cell == Cell::Text));
        assert_eq!(d.period(), 318);
    }
    #[test]
    fn spaces_have_two_thirds_advance_and_text_is_normalized() {
        let mut d = Display::default();
        d.resize(56);
        d.set_text("h h");
        assert_eq!(d.period(), 2 * LETTER_PITCH + SPACE_PITCH + 18);
        d.compose(0., 0, [0.; 3], [0.5; 3]);
        assert_eq!(d.cells[4 * 56 + LETTER_PITCH + SPACE_PITCH], Cell::Text);
        assert!(d.cells[4 * 56 + 15..4 * 56 + LETTER_PITCH + SPACE_PITCH]
            .iter()
            .all(|cell| *cell != Cell::Text));
        assert_eq!(
            normalize_message("dream it. café 🦀\n@build?"),
            "DREAM IT. CAF BUILD?"
        );
        assert_eq!(normalize_message(&"a".repeat(300)).len(), 256);
        d.set_text("🦀");
        assert_eq!(d.period(), 18);
    }
    #[test]
    fn headline_is_centered_bold_and_overwrites_waves() {
        let mut d = Display::default();
        d.resize(56);
        d.set_text("H");
        d.compose(0., 0, [1.; 3], [0.5; 3]);
        let rows: Vec<_> = d
            .cells
            .chunks(d.columns)
            .enumerate()
            .filter(|(_, r)| r.contains(&Cell::Text))
            .map(|(y, _)| y)
            .collect();
        assert_eq!(rows, (4..25).collect::<Vec<_>>());
        for y in 4..25 {
            assert_eq!(&d.cells[y * 56..y * 56 + 3], &[Cell::Text; 3]);
            assert_eq!(&d.cells[y * 56 + 12..y * 56 + 15], &[Cell::Text; 3]);
        }
        for y in 13..16 {
            assert_eq!(&d.cells[y * 56..y * 56 + 15], &[Cell::Text; 15]);
        }
        assert_ne!(d.cells[4 * 56 + 3], Cell::Text);
        assert_ne!(d.cells[4 * 56 + 11], Cell::Text);
        assert_ne!(d.cells[4 * 56 + 15], Cell::Text);
    }
    #[test]
    fn zero_amplitude_uses_one_shared_baseline() {
        let mut d = Display::default();
        d.resize(96);
        d.set_text("");
        d.compose(3.2, 0, [0.; 3], [0.5; 3]);
        for (i, c) in d.cells.iter().enumerate() {
            if *c != Cell::Off {
                assert_eq!(i / 96, 14);
            }
        }
    }
    #[test]
    fn marquee_wrap_and_narrow_crop_are_exact() {
        let mut a = Display::default();
        a.resize(56);
        a.set_text("DREAM IT. BUILD IT.");
        a.compose(1.3, 5, [0.7; 3], [0.5; 3]);
        let expected = a.cells.clone();
        a.compose(1.3, 5 + a.period(), [0.7; 3], [0.5; 3]);
        assert_eq!(a.cells, expected);
        let mut b = Display::default();
        b.resize(28);
        b.set_text("DREAM IT. BUILD IT.");
        b.compose(1.3, 5, [0.7; 3], [0.5; 3]);
        for y in 0..ROWS {
            assert_eq!(
                &a.cells[y * 56..y * 56 + 28],
                &b.cells[y * 28..(y + 1) * 28]
            );
        }
    }
}
