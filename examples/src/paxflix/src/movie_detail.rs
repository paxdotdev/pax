#![allow(unused_imports)]

use crate::Movie;
use crate::{CinemaTheme, FilmArtwork};
use pax_kit::*;

#[pax]
#[file("movie_detail.pax")]
pub struct MovieDetail {
    pub light_mode: Property<bool>,
    pub movie: Property<Movie>,
    pub modal_open: Property<bool>,
    pub feedback: Property<String>,
}

impl MovieDetail {
    pub fn close(&mut self, _ctx: &NodeContext, _event: Event<Click>) {
        self.modal_open.set(false);
    }
    pub fn play(&mut self, _ctx: &NodeContext, _event: Event<Click>) {
        self.feedback
            .set("You found your next watch. Playback is a demo.".into());
    }
    pub fn download(&mut self, _ctx: &NodeContext, _event: Event<Click>) {
        self.feedback
            .set("Saved for an imaginary journey. Download is a demo.".into());
    }
}
