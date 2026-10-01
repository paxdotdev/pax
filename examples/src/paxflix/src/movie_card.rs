#![allow(unused_imports)]

use crate::movie_shelf::CardEntrance;
use crate::CinemaTheme;
use crate::Movie;
use pax_kit::pax_engine::api::cursor::CursorStyle;
use pax_kit::*;
use std::cell::Cell;

#[pax]
#[file("movie_card.pax")]
pub struct MovieCard {
    pub light_mode: Property<bool>,
    pub movie: Property<Movie>,
    pub index: Property<usize>,
    pub entrances: Property<Vec<CardEntrance>>,
    pub selected_id: Property<usize>,
    pub modal_open: Property<bool>,
    pub hover: Property<bool>,
    pub reveal_progress: Property<f64>,
    pub revealed: Property<bool>,
    pub reveal_visit: Property<u64>,
}

impl MovieCard {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        let entrances = self.entrances.local();
        let index = self.index.local();
        let revealed = self.revealed.local();
        let visit = self.reveal_visit.local();
        let progress = self.reveal_progress.local();
        let now = ctx.elapsed_millis.clone();
        let last_started = Cell::new(0);
        ctx.subscribe(&[entrances.untyped()], move || {
            if !revealed.get() || last_started.get() == visit.get() {
                return;
            }
            let index = index.get();
            let visit = visit.get();
            let start = entrances.read(|entries| {
                entries
                    .iter()
                    .find(|entry| entry.index == index && entry.visit == visit && !entry.pending)
                    .map(|entry| entry.start_at)
            });
            if let Some(start) = start {
                last_started.set(visit);
                let delay = start.saturating_sub(now.get());
                progress.cancel_transitions();
                progress.set(0.0);
                if delay > 0 {
                    progress.ease_to(
                        0.0,
                        Duration::Milliseconds(delay.into()),
                        EasingCurve::Linear,
                    );
                }
                progress.ease_to_later(
                    100.0,
                    Duration::Milliseconds(480.into()),
                    EasingCurve::Linear,
                );
            }
        });
    }

    pub fn reveal(&mut self, ctx: &NodeContext, event: Event<ViewportProximityChange>) {
        // Request entry at the first sampled positive overlap, not an area threshold.
        // The fixed wrapper keeps the text's travel out of this measurement.
        if event.current.is_in_viewport() && !self.revealed.get() {
            self.revealed.set(true);
            let visit = self.reveal_visit.get() + 1;
            self.reveal_visit.set(visit);
            let index = self.index.get();
            self.entrances.update(|entries| {
                entries.retain(|entry| entry.index != index);
                entries.push(CardEntrance {
                    index,
                    visit,
                    arrived_at: ctx.elapsed_millis.get(),
                    start_at: 0,
                    pending: true,
                });
            });
        }
    }

    pub fn reset_reveal(&mut self, _ctx: &NodeContext, _event: Event<ViewportProximityExit>) {
        // Rearm only once well offscreen, without an exit animation or reloading artwork.
        self.reveal_progress.local().cancel_transitions();
        self.reveal_progress.set(0.0);
        self.revealed.set(false);
        self.hover.set(false);
        self.forget_entrance();
    }

    pub fn on_unmount(&mut self, _ctx: &NodeContext) {
        self.reveal_progress.local().cancel_transitions();
        self.forget_entrance();
    }

    fn forget_entrance(&self) {
        let index = self.index.get();
        if self
            .entrances
            .read(|entries| entries.iter().any(|entry| entry.index == index))
        {
            // Discard unstarted reservations too, so an offscreen card cannot
            // keep holding a future slot after its visit ends.
            self.entrances
                .update(|entries| entries.retain(|entry| entry.index != index));
        }
    }

    pub fn open(&mut self, _ctx: &NodeContext, _event: Event<Click>) {
        if !self.modal_open.get() {
            self.selected_id.set(self.movie.get().id);
            self.modal_open.set(true);
        }
    }
    pub fn over(&mut self, ctx: &NodeContext, _event: Event<MouseOver>) {
        ctx.set_cursor(CursorStyle::Pointer);
        self.hover.set(true);
    }
    pub fn out(&mut self, ctx: &NodeContext, _event: Event<MouseOut>) {
        ctx.set_cursor(CursorStyle::Auto);
        self.hover.set(false);
    }
}
