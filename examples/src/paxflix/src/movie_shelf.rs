#![allow(unused_imports)]

use crate::CinemaTheme;
use crate::{MovieCard, Shelf};
use pax_kit::*;

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
}

impl MovieShelf {
    pub fn next(&mut self, ctx: &NodeContext, _event: Event<Click>) {
        if !self.modal_open.get() {
            let max = (self.card_width.get() + 16.0) * 10.0 + self.gutter.get() * 2.0
                - 16.0
                - ctx.bounds_self.get().0;
            self.scroll_x.ease_to(
                (self.scroll_x.get() + (self.card_width.get() + 16.0) * 2.0).min(max.max(0.0)),
                Duration::Milliseconds(360.into()),
                EasingCurve::OutQuad,
            );
        }
    }
    pub fn previous(&mut self, _ctx: &NodeContext, _event: Event<Click>) {
        if !self.modal_open.get() {
            self.scroll_x.ease_to(
                (self.scroll_x.get() - (self.card_width.get() + 16.0) * 2.0).max(0.0),
                Duration::Milliseconds(360.into()),
                EasingCurve::OutQuad,
            );
        }
    }

    pub fn wheel(&mut self, _ctx: &NodeContext, _event: Event<Wheel>) {
        self.scroll_x.cancel_transitions();
    }

    pub fn touch_start(&mut self, _ctx: &NodeContext, _event: Event<TouchStart>) {
        self.scroll_x.cancel_transitions();
    }
}
