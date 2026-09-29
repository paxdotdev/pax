#![allow(unused_imports)]

use crate::CinemaTheme;
use crate::Movie;
use pax_kit::*;

#[pax]
#[file("movie_card.pax")]
pub struct MovieCard {
    pub light_mode: Property<bool>,
    pub movie: Property<Movie>,
    pub selected_id: Property<usize>,
    pub modal_open: Property<bool>,
    pub hover: Property<bool>,
    pub reveal_progress: Property<f64>,
    pub revealed: Property<bool>,
}

impl MovieCard {
    pub fn reveal(&mut self, _ctx: &NodeContext, event: Event<ViewportProximityChange>) {
        if event.current.is_in_viewport() && !self.revealed.get() {
            self.revealed.set(true);
            self.reveal_progress.ease_to(
                100.0,
                Duration::Milliseconds(420.into()),
                EasingCurve::Linear,
            );
        }
    }

    pub fn reset_reveal(&mut self, _ctx: &NodeContext, _event: Event<ViewportProximityExit>) {
        // Rearm only once well offscreen, without an exit animation or reloading artwork.
        self.reveal_progress.cancel_transitions();
        self.reveal_progress.set(0.0);
        self.revealed.set(false);
        self.hover.set(false);
    }

    pub fn open(&mut self, _ctx: &NodeContext, _event: Event<Click>) {
        if !self.modal_open.get() {
            self.selected_id.set(self.movie.get().id);
            self.modal_open.set(true);
        }
    }
    pub fn over(&mut self, _ctx: &NodeContext, _event: Event<MouseOver>) {
        self.hover.set(true);
    }
    pub fn out(&mut self, _ctx: &NodeContext, _event: Event<MouseOut>) {
        self.hover.set(false);
    }
}
