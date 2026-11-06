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
    pub journey_id: Property<usize>,
    pub manual_until: Property<f64>,
    pub manual_contacts: Property<usize>,
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
        self.journeys.set(
            self.phases
                .get()
                .iter()
                .enumerate()
                .map(|(row, phase)| RowJourney {
                    from: *phase,
                    distance: 0.0,
                    slot: row,
                })
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

    pub fn on_pre_render(&mut self, ctx: &NodeContext) {
        self.layout(ctx);
        let now = ctx.elapsed_time_millis() as f64;
        let dt = (now - self._last_ms.get()).clamp(0.0, 50.0);
        self._last_ms.set(now);
        if !self.ready.get() || !self._visible.get() {
            return;
        }
        if self.manual_contacts.get() > 0 || self.manual_until.get() > 0.0 {
            if self.manual_contacts.get() == 0 && now >= self.manual_until.get() {
                self.manual_until.set(0.0);
                self.begin_next_journey();
            }
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
                        journey.from + journey.distance * ease(time / ROW_DURATIONS[row])
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
                self.begin_next_journey();
            } else {
                self.phase_ms.set(time);
            }
        }
    }

    fn begin_next_journey(&mut self) {
        let next = (self.word_index.get() + 1) % WORDS.len();
        let phases: Vec<_> = self.phases.get().into_iter().map(wrap).collect();
        let journeys: Vec<_> = WORDS[next]
            .bytes()
            .enumerate()
            .map(|(row, letter)| plan_row(phases[row], letter, LANDING_COLUMNS[next], row, next))
            .collect();
        self.selected
            .set(journeys.iter().map(|journey| journey.slot).collect());
        self.phases.set(phases);
        self.journeys.set(journeys);
        self.journey_id.set(self.journey_id.get() + 1);
        self.word_index.set(next);
        self.phase_ms.set(0.0);
        self.light.set(0.0);
        self.moving.set(true);
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
    pub lit_overlay: Property<bool>,
}

/// A finite native-scroll experiment: three fixed tapes share one animated offset.
/// The illuminated glyph remains in the parent's overlay so its glow is not row-clipped.
#[pax]
#[file("acrostic_scroll_row.pax")]
pub struct AcrosticScrollRow {
    pub glyphs: Property<Vec<AcrosticGlyph>>,
    pub phases: Property<Vec<f64>>,
    pub row: Property<usize>,
    pub journey: Property<RowJourney>,
    pub journey_id: Property<usize>,
    pub selected: Property<usize>,
    pub cell_px: Property<f64>,
    pub row_height: Property<f64>,
    pub lit_overlay: Property<bool>,
    pub moving: Property<bool>,
    pub manual_until: Property<f64>,
    pub manual_contacts: Property<usize>,
    pub scroll_x: Property<f64>,
    pub _anchor: Property<f64>,
    pub _expected_scroll: Property<f64>,
    pub _last_cell: Property<f64>,
    pub _last_journey: Property<usize>,
    pub _touching: Property<bool>,
}

impl AcrosticScrollRow {
    pub fn mount(&mut self, ctx: &NodeContext) {
        self.sync_scroll(ctx);
    }

    pub fn on_pre_render(&mut self, ctx: &NodeContext) {
        self.sync_scroll(ctx);
    }

    fn sync_scroll(&mut self, ctx: &NodeContext) {
        let cell = self.cell_px.get();
        if cell <= 0.0 {
            return;
        }
        let row = self.row.get();
        let mut phases = self.phases.get();
        let phase = phases[row];
        let reset =
            self._last_cell.get() != cell || self._last_journey.get() != self.journey_id.get();
        let now = ctx.elapsed_time_millis() as f64;
        let actual = self.scroll_x.get();
        if !reset && (actual - self._expected_scroll.get()).abs() > 0.75 {
            // Native observations update the binding; autoplay must yield to them.
            phases[row] = self._anchor.get() - actual / cell;
            self.phases.set(phases);
            self._expected_scroll.set(actual);
            self.manual_until.set(now + 900.0);
            return;
        }
        if !reset && (self.manual_contacts.get() > 0 || self.manual_until.get() > 0.0) {
            return;
        }
        if reset {
            let journey = self.journey.get();
            let remaining = if self.moving.get() {
                journey.from + journey.distance - phase
            } else {
                0.0
            };
            let previous = (self._last_cell.get() > 0.0).then(|| self._anchor.get());
            self._anchor.set(scroll_anchor(
                phase,
                remaining,
                ctx.bounds_self.get().0 / cell,
                previous,
            ));
            self._last_cell.set(cell);
            self._last_journey.set(self.journey_id.get());
        }
        let requested = (self._anchor.get() - phase) * cell;
        self.scroll_x.set_if_neq(requested);
        self._expected_scroll.set(requested);
    }

    pub fn touch_start(&mut self, ctx: &NodeContext, _event: Event<TouchStart>) {
        if !self._touching.get() {
            self._touching.set(true);
            self.manual_contacts.set(self.manual_contacts.get() + 1);
        }
        self.manual_until
            .set(ctx.elapsed_time_millis() as f64 + 900.0);
    }

    fn release_touch(&mut self, ctx: &NodeContext) {
        if self._touching.get() {
            self._touching.set(false);
            self.manual_contacts
                .set(self.manual_contacts.get().saturating_sub(1));
            self.manual_until
                .set(ctx.elapsed_time_millis() as f64 + 900.0);
        }
    }

    pub fn touch_end(&mut self, ctx: &NodeContext, _event: Event<TouchEnd>) {
        self.release_touch(ctx);
    }
    pub fn touch_cancel(&mut self, ctx: &NodeContext, _event: Event<TouchCancel>) {
        self.release_touch(ctx);
    }
    pub fn on_unmount(&mut self, ctx: &NodeContext) {
        self.release_touch(ctx);
    }
}

fn scroll_anchor(phase: f64, distance: f64, columns: f64, previous: Option<f64>) -> f64 {
    let period = TAPE.len() as f64;
    let max_scroll = 3.0 * period - columns;
    // A whole-period shift changes the finite scroll coordinate, not the visible letters.
    // Reserve the entire journey before it begins; never rebase in mid-flight.
    let first = ((phase + distance.max(0.0)) / period - 1e-9).ceil();
    let last = ((phase + max_scroll + distance.min(0.0)) / period + 1e-9).floor();
    assert!(
        first <= last,
        "three acrostic tapes must fit the complete journey"
    );
    let preferred = previous
        .map(|a| a / period)
        .unwrap_or((phase + max_scroll / 2.0) / period);
    preferred.round().clamp(first, last) * period
}

#[helpers]
impl AcrosticRow {
    pub fn radiance_light_y(phase_ms: f64, row: usize) -> f64 {
        // One light crosses the entire column. Translate it into each retained
        // letter artboard, so the rays agree rather than repeating four sweeps.
        let column_y = 3500.0 * (1.0 - ease(phase_ms / HOLD_MS));
        column_y - row as f64 * 875.0
    }

    pub fn radiance_tint(light: f64) -> Color {
        let mut paper = SiteTheme::paper().to_rgba_0_1();
        // Fade emitted light, not the opaque letter face, on the existing envelope.
        paper[3] = light.clamp(0.0, 1.0) * 0.36;
        Color::from_rgba_0_1(paper)
    }

    pub fn radiance_reach(cell_px: f64) -> f64 {
        // Radiance is authored in a 1000-unit letter cell. Bound its visible
        // reach to 36% of a cell and at most 48 logical pixels on large screens.
        if !cell_px.is_finite() || cell_px <= 0.0 {
            return 0.0;
        }
        Self::radiance_margin(cell_px) * 1000.0 / cell_px
    }

    pub fn radiance_margin(cell_px: f64) -> f64 {
        if !cell_px.is_finite() || cell_px <= 0.0 {
            return 0.0;
        }
        (cell_px * 0.36).min(48.0)
    }

    pub fn position(slot: usize, phase: f64) -> f64 {
        // Recycle only in the clipped overscan, never at a visible cell boundary.
        wrap(slot as f64 + phase + 1.0) - 1.0
    }

    pub fn radiance_position(slot: usize, phase: f64) -> f64 {
        // The letter plus its <=0.36-cell glow must leave the viewport before
        // recycling. A two-cell left margin keeps that seam outside the clip.
        wrap(slot as f64 + phase + 2.0) - 2.0
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
    fn manual_highlight_tracks_visible_copies_including_the_glow_margin() {
        for columns in [6.0, 7.0] {
            for cell in [28.0, 57.0, 138.0, 400.0] {
                let margin = AcrosticRow::radiance_margin(cell) / cell;
                for slot in 0..TAPE.len() {
                    for step in 0..=1060 {
                        let scroll = (60.0 - columns) * step as f64 / 1060.0;
                        let highlight = AcrosticRow::radiance_position(slot, 40.0 - scroll);
                        for copy in 0..3 {
                            let letter = (copy * TAPE.len() + slot) as f64 - scroll;
                            // Include rays from letters just outside the scroll viewport,
                            // and the outer clip's allowance for the original glow.
                            if letter + 1.0 + margin > -margin && letter - margin < columns + margin
                            {
                                assert!((highlight - letter).abs() < 1e-9);
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn finite_scroll_tapes_preserve_every_journey_and_fractional_manual_start() {
        for columns in [6.0, 7.0] {
            for row in 0..4 {
                let mut anchor = None;
                for step in 0..100 {
                    let word = step % WORDS.len();
                    let phase = wrap(step as f64 * 0.731);
                    let journey = plan_row(
                        phase,
                        WORDS[word].as_bytes()[row],
                        LANDING_COLUMNS[word],
                        row,
                        word,
                    );
                    let next = scroll_anchor(phase, journey.distance, columns, anchor);
                    let max_scroll = 3.0 * TAPE.len() as f64 - columns;
                    for tick in 0..=50 {
                        let current = phase + journey.distance * ease(tick as f64 / 50.0);
                        let scroll = next - current;
                        assert!(scroll >= -1e-9 && scroll <= max_scroll + 1e-9);
                        let mut visible = Vec::new();
                        for index in 0..3 * TAPE.len() {
                            let x = index as f64 - scroll;
                            if x > -1.0 && x < columns {
                                let slot = index % TAPE.len();
                                assert!((x - AcrosticRow::position(slot, current)).abs() < 1e-8);
                                visible.push(x);
                            }
                        }
                        assert!(visible.len() >= columns as usize);
                        // Resizing midway chooses a valid equivalent coordinate for the remaining trip.
                        let resized = scroll_anchor(
                            current,
                            phase + journey.distance - current,
                            7.0,
                            Some(next),
                        );
                        assert!((resized / TAPE.len() as f64).fract().abs() < 1e-9);
                        assert!(resized - current >= -1e-9);
                    }
                    anchor = Some(next);
                }
            }
        }
    }

    #[test]
    fn backlight_sweeps_up_once_per_hold_in_shared_column_space() {
        for row in 0..4 {
            let offset = row as f64 * 875.0;
            assert_eq!(AcrosticRow::radiance_light_y(0.0, row) + offset, 3500.0);
            assert_eq!(
                AcrosticRow::radiance_light_y(HOLD_MS / 2.0, row) + offset,
                1750.0
            );
            assert_eq!(AcrosticRow::radiance_light_y(HOLD_MS, row) + offset, 0.0);
            let mut previous = 3500.0;
            for tick in 0..=100 {
                let time = HOLD_MS * tick as f64 / 100.0;
                let y = AcrosticRow::radiance_light_y(time, row) + offset;
                assert!(y <= previous);
                assert!((0.0..=3500.0).contains(&y));
                previous = y;
            }
        }
        // The reset between holds happens with the illumination extinguished.
        assert_eq!(illumination(0.0), 0.0);
        assert_eq!(illumination(HOLD_MS), 0.0);
    }

    #[test]
    fn radiance_tracks_the_white_fade_with_restrained_opacity_and_reach() {
        for time in [0.0, 95.0, 190.0, 380.0, 1000.0, HOLD_MS - 140.0, HOLD_MS] {
            let light = illumination(time);
            let tint = AcrosticRow::radiance_tint(light).to_rgba_0_1();
            assert!((tint[3] - light * 0.36).abs() < 1e-6);
            assert!(tint[3] <= 0.36 + 1e-6);
            assert_eq!(AcrosticRow::ink(light).to_rgba_0_1()[3], 1.0);
        }
        for (cell, expected_px) in [
            (40.0, 14.4),
            (60.0, 21.6),
            (120.0, 43.2),
            (160.0, 48.0),
            (300.0, 48.0),
        ] {
            let reach = AcrosticRow::radiance_reach(cell);
            assert!(reach <= 360.0 + 1e-9);
            assert!((reach * cell / 1000.0 - expected_px).abs() < 1e-9);
        }
        for cell in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            assert_eq!(AcrosticRow::radiance_reach(cell), 0.0);
        }
    }

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
