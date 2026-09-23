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
}

impl MovieCard {
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
