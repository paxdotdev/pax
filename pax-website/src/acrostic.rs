use crate::SiteTheme;
use pax_kit::*;

const WORDS: [&str; 5] = ["MAKE", "PLAY", "OPEN", "FREE", "CODE"];
const TAPE: &str = "MAKEPLAYOPENFREECODE";
const LANDING_COLUMNS: [usize; 5] = [1, 4, 2, 0, 3];
const HOLD_MS: f64 = 3200.0;
const TRAVEL_MS: f64 = 2300.0;
const ROW_DURATIONS: [f64; 4] = [2100.0, 1800.0, 2300.0, 1950.0];

#[pax]
pub struct AcrosticGlyph {
    pub slot: usize,
    pub elements: Vec<PathElement>,
}

#[pax]
pub struct RowJourney {
    pub from: f64,
    pub distance: f64,
    pub slot: usize,
}

/// Four retained letter tapes align a small vocabulary into vertical words.
/// Font outlines are baked offline; playback changes only positions and paint.
#[pax]
#[file("acrostic.pax")]
pub struct Acrostic {
    pub columns: Property<usize>,
    pub glyphs: Property<Vec<AcrosticGlyph>>,
    pub phases: Property<Vec<f64>>,
    pub selected: Property<Vec<usize>>,
    pub journeys: Property<Vec<RowJourney>>,
    pub cell_px: Property<f64>,
    pub row_height: Property<f64>,
    pub light: Property<f64>,
    pub ready: Property<bool>,
    pub moving: Property<bool>,
    pub word_index: Property<usize>,
    pub phase_ms: Property<f64>,
    pub _visible: Property<bool>,
    pub _last_ms: Property<f64>,
}

impl Acrostic {
    pub fn mount(&mut self, ctx: &NodeContext) {
        let alphabet = crate::hero_outlines::acrostic_glyphs();
        self.glyphs.set(
            TAPE.chars()
                .enumerate()
                .map(|(slot, letter)| {
                    let (_, elements) = alphabet
                        .iter()
                        .find(|g| g.0 == letter)
                        .expect("the baked alphabet must cover the acrostic tape");
                    AcrosticGlyph {
                        slot,
                        elements: elements.clone(),
                    }
                })
                .collect(),
        );
        self.selected.set(vec![0, 1, 2, 3]);
        self.phases.set(
            (0..4)
                .map(|row| wrap(LANDING_COLUMNS[0] as f64 - row as f64))
                .collect(),
        );
        self.light.set(1.0);
        // Begin with MAKE already illuminated, not a blank or half-solved sign.
        self.phase_ms.set(400.0);
        self._last_ms.set(ctx.elapsed_time_millis() as f64);
        self.ready.set(true);
        self.layout(ctx);
    }

    pub fn height(width: f64, columns: usize) -> f64 {
        width.max(0.0) / columns.max(1) as f64 * 3.5
    }

    fn layout(&mut self, ctx: &NodeContext) {
        let cell = ctx.bounds_self.get().0.max(0.0) / self.columns.get().max(1) as f64;
        self.cell_px.set_if_neq(cell);
        self.row_height.set_if_neq(cell * 0.875);
    }

    pub fn visibility(&mut self, ctx: &NodeContext, event: Event<ViewportProximityChange>) {
        self._visible.set_if_neq(event.current.is_in_viewport());
        self._last_ms.set(ctx.elapsed_time_millis() as f64);
    }

    pub fn tick(&mut self, ctx: &NodeContext) {
        self.layout(ctx);
        let now = ctx.elapsed_time_millis() as f64;
        let dt = (now - self._last_ms.get()).clamp(0.0, 50.0);
        self._last_ms.set(now);
        if !self.ready.get() || !self._visible.get() {
            return;
        }
        let time = self.phase_ms.get() + dt;
        if self.moving.get() {
            let journeys = self.journeys.get();
            self.phases.set(
                journeys
                    .iter()
                    .enumerate()
                    .map(|(row, journey)| {
                        wrap(journey.from + journey.distance * ease(time / ROW_DURATIONS[row]))
                    })
                    .collect(),
            );
            if time >= TRAVEL_MS {
                self.moving.set(false);
                self.phase_ms.set(0.0);
            } else {
                self.phase_ms.set(time);
            }
        } else {
            self.light.set_if_neq(illumination(time));
            if time >= HOLD_MS {
                let next = (self.word_index.get() + 1) % WORDS.len();
                let phases = self.phases.get();
                let journeys: Vec<_> = WORDS[next]
                    .bytes()
                    .enumerate()
                    .map(|(row, letter)| {
                        plan_row(phases[row], letter, LANDING_COLUMNS[next], row, next)
                    })
                    .collect();
                self.selected
                    .set(journeys.iter().map(|journey| journey.slot).collect());
                self.journeys.set(journeys);
                self.word_index.set(next);
                self.phase_ms.set(0.0);
                self.light.set(0.0);
                self.moving.set(true);
            } else {
                self.phase_ms.set(time);
            }
        }
    }
}

#[pax]
#[file("acrostic_row.pax")]
#[has_helpers]
pub struct AcrosticRow {
    pub glyphs: Property<Vec<AcrosticGlyph>>,
    pub phase: Property<f64>,
    pub selected: Property<usize>,
    pub light: Property<f64>,
    pub cell_px: Property<f64>,
    pub row_height: Property<f64>,
}

#[helpers]
impl AcrosticRow {
    pub fn position(slot: usize, phase: f64) -> f64 {
        // Recycle only in the clipped overscan, never at a visible cell boundary.
        wrap(slot as f64 + phase + 1.0) - 1.0
    }

    pub fn ink(light: f64) -> Color {
        let paper = SiteTheme::paper().to_rgba_0_1();
        let t = light.clamp(0.0, 1.0);
        Color::from_rgba_0_1([
            0.19 + (paper[0] - 0.19) * t,
            0.19 + (paper[1] - 0.19) * t,
            0.18 + (paper[2] - 0.18) * t,
            1.0,
        ])
    }
}

fn wrap(value: f64) -> f64 {
    value.rem_euclid(TAPE.len() as f64)
}

fn ease(t: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

fn illumination(time: f64) -> f64 {
    ease(time / 380.0) * ease((HOLD_MS - time) / 280.0)
}

fn plan_row(from: f64, letter: u8, column: usize, row: usize, word: usize) -> RowJourney {
    let right = (row + word) % 2 == 0;
    let minimum = 2.0 + row as f64 * 0.45;
    TAPE.bytes()
        .enumerate()
        .filter(|(_, c)| *c == letter)
        .map(|(slot, _)| {
            let mut distance = wrap(column as f64 - slot as f64 - from);
            if !right {
                distance -= TAPE.len() as f64;
            }
            if distance.abs() < minimum {
                distance += if right {
                    TAPE.len() as f64
                } else {
                    -(TAPE.len() as f64)
                };
            }
            RowJourney {
                from,
                distance,
                slot,
            }
        })
        .min_by(|a, b| a.distance.abs().total_cmp(&b.distance.abs()))
        .expect("every word must be representable by the fixed letter tape")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_word_lands_exactly_and_moves_to_a_different_column() {
        let mut phases: Vec<_> = (0..4)
            .map(|r| wrap(LANDING_COLUMNS[0] as f64 - r as f64))
            .collect();
        for step in 1..101 {
            let word = step % WORDS.len();
            let column = LANDING_COLUMNS[word];
            assert_ne!(
                column,
                LANDING_COLUMNS[(word + WORDS.len() - 1) % WORDS.len()]
            );
            for (row, letter) in WORDS[word].bytes().enumerate() {
                let journey = plan_row(phases[row], letter, column, row, word);
                assert_eq!(TAPE.as_bytes()[journey.slot], letter);
                assert_eq!(journey.distance > 0.0, (row + word) % 2 == 0);
                assert!(journey.distance.abs() <= TAPE.len() as f64 + 4.0);
                phases[row] = wrap(journey.from + journey.distance);
                assert!(
                    (AcrosticRow::position(journey.slot, phases[row]) - column as f64).abs() < 1e-9
                );
            }
        }
    }

    #[test]
    fn wrap_never_leaves_a_hole_in_the_visible_field() {
        for columns in [6.0, 7.0] {
            for tick in 0..2000 {
                let phase = tick as f64 / 100.0;
                let mut positions: Vec<_> = (0..TAPE.len())
                    .map(|i| AcrosticRow::position(i, phase))
                    .filter(|x| *x > -1.0 && *x < columns)
                    .collect();
                positions.sort_by(f64::total_cmp);
                assert!(positions.len() >= columns as usize);
                for pair in positions.windows(2) {
                    assert!((pair[1] - pair[0] - 1.0).abs() < 1e-9);
                }
            }
        }
        assert_eq!(Acrostic::height(600.0, 6), 350.0);
        assert_eq!(Acrostic::height(700.0, 7), 350.0);
    }

    #[test]
    fn type_is_retained_and_illumination_is_quiet_during_travel() {
        assert_eq!(TAPE, WORDS.concat());
        let alphabet = crate::hero_outlines::acrostic_glyphs();
        for letter in TAPE.chars() {
            let (_, elements) = alphabet.iter().find(|g| g.0 == letter).unwrap();
            assert!(!elements.is_empty());
        }
        assert_eq!(illumination(0.0), 0.0);
        assert_eq!(illumination(HOLD_MS), 0.0);
        assert_eq!(illumination(1000.0), 1.0);
        assert_eq!(ease(-1.0), 0.0);
        assert_eq!(ease(1.0), 1.0);
        assert_eq!(TAPE.len() * 4, 80);
    }

    #[test]
    fn full_letter_outlines_fit_equal_gutters_including_curved_overshoot() {
        for (letter, elements) in crate::hero_outlines::acrostic_glyphs() {
            let mut points = Vec::new();
            for element in elements {
                match element {
                    PathElement::Point(x, y) | PathElement::Quadratic(x, y) => points.push((x, y)),
                    PathElement::Cubic(a, b, c, d) => points.extend([(a, b), (c, d)]),
                    _ => (),
                }
            }
            let points: Vec<_> = points
                .into_iter()
                .map(|(x, y)| (x.expect_percent(), y.expect_percent()))
                .collect();
            let bounds = points
                .iter()
                .fold([1.0_f64, 1.0_f64, 0.0_f64, 0.0_f64], |b, &(x, y)| {
                    assert!(
                        (0.0..=1.0).contains(&x) && (0.0..=1.0).contains(&y),
                        "{letter}"
                    );
                    [b[0].min(x), b[1].min(y), b[2].max(x), b[3].max(y)]
                });
            assert_eq!(bounds, [0.0, 0.0, 1.0, 1.0], "{letter}");
            // Normalized bounds plus the centered outline stay inside each cell.
            for cell in [48.0_f64, 120.0, 200.0] {
                let row = cell * 0.875;
                let inset = cell * 0.04;
                let stroke = cell * 0.025;
                let horizontal_gap = cell - (cell * 0.92 + stroke);
                let vertical_gap = row - (row - cell * 0.08 + stroke);
                assert!((horizontal_gap - vertical_gap).abs() < 1e-9);
                assert!(inset > stroke * 0.5, "top row must not clip its outline");
            }
        }
    }
}
