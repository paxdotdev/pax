#![allow(unused_imports)]

use crate::CinemaTheme;
use crate::{MovieCard, Shelf};
use pax_kit::pax_engine::api::cursor::CursorStyle;
use pax_kit::*;

const ENTRANCE_SPACING_MS: u64 = 80;
const MAX_ENTRANCE_DELAY_MS: u64 = 240;

#[pax]
pub struct CardEntrance {
    pub index: usize,
    pub visit: u64,
    pub arrived_at: u64,
    pub start_at: u64,
    pub pending: bool,
}

// Reserve starts rather than delaying by absolute catalogue index: a lone card
// enters immediately once the previous slot is at least one spacing behind us.
fn schedule_entrances(entries: &mut [CardEntrance], now: u64) {
    let mut last_start = entries
        .iter()
        .filter(|entry| !entry.pending)
        .map(|entry| entry.start_at)
        .max();
    let mut pending: Vec<_> = entries
        .iter()
        .enumerate()
        .filter_map(|(slot, entry)| entry.pending.then_some(slot))
        .collect();
    // Callback order is not a visual-order contract. The shared clock groups
    // arrivals from one sampled frame; their shelf indices order that group.
    pending.sort_unstable_by_key(|&slot| (entries[slot].arrived_at, entries[slot].index));
    for slot in pending {
        let start = last_start
            .map_or(now, |last| last.saturating_add(ENTRANCE_SPACING_MS))
            .max(now)
            .min(now.saturating_add(MAX_ENTRANCE_DELAY_MS));
        entries[slot].start_at = start;
        entries[slot].pending = false;
        last_start = Some(start);
    }
}

#[pax]
#[file("movie_shelf.pax")]
pub struct MovieShelf {
    pub light_mode: Property<bool>,
    pub shelf: Property<Shelf>,
    pub selected_id: Property<usize>,
    pub modal_open: Property<bool>,
    pub card_width: Property<f64>,
    pub gutter: Property<f64>,
    pub scroll_x: Property<f64>,
    pub entrances: Property<Vec<CardEntrance>>,
}

impl MovieShelf {
    pub fn button_over(&mut self, ctx: &NodeContext, _event: Event<MouseOver>) {
        ctx.set_cursor(CursorStyle::Pointer);
    }

    pub fn button_out(&mut self, ctx: &NodeContext, _event: Event<MouseOut>) {
        ctx.set_cursor(CursorStyle::Auto);
    }

    pub fn on_mount(&mut self, ctx: &NodeContext) {
        let entrances = self.entrances.clone();
        let now = ctx.elapsed_millis.clone();
        ctx.subscribe(&[entrances.untyped()], move || {
            // Proximity handlers enqueue a whole sampled batch before effects
            // drain. Publishing starts dirties this effect once more; with no
            // pending requests it is a no-op, not another scheduling pass.
            if entrances.read(|entries| entries.iter().any(|entry| entry.pending)) {
                entrances.update(|entries| schedule_entrances(entries, now.get()));
            }
        });
    }

    pub fn next(&mut self, ctx: &NodeContext, _event: Event<Click>) {
        if !self.modal_open.get() {
            let max = (self.card_width.get() + 16.0) * 10.0 + self.gutter.get() * 2.0
                - 16.0
                - ctx.bounds_self.get().0;
            self.scroll_x.local().ease_to(
                (self.scroll_x.get() + (self.card_width.get() + 16.0) * 2.0).min(max.max(0.0)),
                Duration::Milliseconds(360.into()),
                EasingCurve::OutQuad,
            );
        }
    }
    pub fn previous(&mut self, _ctx: &NodeContext, _event: Event<Click>) {
        if !self.modal_open.get() {
            self.scroll_x.local().ease_to(
                (self.scroll_x.get() - (self.card_width.get() + 16.0) * 2.0).max(0.0),
                Duration::Milliseconds(360.into()),
                EasingCurve::OutQuad,
            );
        }
    }

    pub fn wheel(&mut self, _ctx: &NodeContext, _event: Event<Wheel>) {
        self.scroll_x.local().cancel_transitions();
    }

    pub fn touch_start(&mut self, _ctx: &NodeContext, _event: Event<TouchStart>) {
        self.scroll_x.local().cancel_transitions();
    }
}

#[cfg(test)]
mod entrance_tests {
    use super::*;

    fn request(index: usize, at: u64) -> CardEntrance {
        CardEntrance {
            index,
            visit: 1,
            arrived_at: at,
            pending: true,
            ..Default::default()
        }
    }

    fn starts(entries: &[CardEntrance]) -> Vec<(usize, u64)> {
        let mut result: Vec<_> = entries
            .iter()
            .map(|entry| (entry.index, entry.start_at))
            .collect();
        result.sort_unstable();
        result
    }

    #[test]
    fn simultaneous_arrivals_stagger_in_visual_order_not_callback_order() {
        let mut entries = vec![request(5, 1000), request(3, 1000), request(4, 1000)];
        schedule_entrances(&mut entries, 1000);
        assert_eq!(starts(&entries), vec![(3, 1000), (4, 1080), (5, 1160)]);
    }

    #[test]
    fn slow_scroll_has_no_added_delay_even_late_in_the_row() {
        let mut entries = vec![request(8, 1000)];
        schedule_entrances(&mut entries, 1000);
        entries.push(request(9, 1250));
        schedule_entrances(&mut entries, 1250);
        assert_eq!(starts(&entries), vec![(8, 1000), (9, 1250)]);
    }

    #[test]
    fn rapid_crossings_across_frames_share_the_same_spacing() {
        let mut entries = vec![request(0, 1000)];
        schedule_entrances(&mut entries, 1000);
        entries.push(request(1, 1030));
        schedule_entrances(&mut entries, 1030);
        entries.push(request(2, 1060));
        schedule_entrances(&mut entries, 1060);
        assert_eq!(starts(&entries), vec![(0, 1000), (1, 1080), (2, 1160)]);
    }

    #[test]
    fn large_bursts_cap_added_delay_and_leave_finished_starts_unchanged() {
        let mut entries: Vec<_> = (0..10).map(|index| request(index, 1000)).collect();
        schedule_entrances(&mut entries, 1000);
        assert_eq!(
            starts(&entries)[..4],
            [(0, 1000), (1, 1080), (2, 1160), (3, 1240)]
        );
        assert!(entries
            .iter()
            .all(|entry| entry.start_at <= 1240 && !entry.pending));
        let reserved = starts(&entries);
        schedule_entrances(&mut entries, 1500);
        assert_eq!(starts(&entries), reserved);
    }

    #[test]
    fn rows_do_not_delay_one_another() {
        let mut first = vec![request(0, 1000), request(1, 1000)];
        let mut second = vec![request(7, 1000)];
        schedule_entrances(&mut first, 1000);
        schedule_entrances(&mut second, 1000);
        assert_eq!(starts(&second), vec![(7, 1000)]);
    }

    #[test]
    fn departed_cards_release_their_future_slots() {
        let mut entries: Vec<_> = (0..4).map(|index| request(index, 1000)).collect();
        schedule_entrances(&mut entries, 1000);
        // Proximity exit/unmount removes the departing cards' reservations.
        entries.retain(|entry| entry.index == 0);
        entries.push(request(8, 1050));
        schedule_entrances(&mut entries, 1050);
        assert_eq!(starts(&entries), vec![(0, 1000), (8, 1080)]);
    }
}
