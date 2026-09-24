use pax_kit::*;
pub mod catalog;
pub mod film_artwork;
pub mod movie_card;
pub mod movie_detail;
pub mod movie_shelf;
pub mod profile_menu;
pub use catalog::*;
pub use film_artwork::*;
pub use movie_card::*;
pub use movie_detail::*;
pub use movie_shelf::*;
pub use profile_menu::*;

#[pax]
#[main]
#[file("lib.pax")]
pub struct Paxflix {
    // The default false value starts the app in dark mode.
    pub light_mode: Property<bool>,
    pub shelves: Property<Vec<Shelf>>,
    pub shelf_count: Property<usize>,
    pub selected_id: Property<usize>,
    pub selected: Property<Movie>,
    pub modal_open: Property<bool>,
    pub profile_open: Property<bool>,
    pub scroll_y: Property<f64>,
    pub compact: Property<bool>,
    pub gutter: Property<f64>,
    pub top_inset: Property<f64>,
    pub safe_left: Property<f64>,
    pub safe_right: Property<f64>,
    pub safe_bottom: Property<f64>,
    pub hero_height: Property<f64>,
    pub card_width: Property<f64>,
    pub row_height: Property<f64>,
}

impl Paxflix {
    pub fn mount(&mut self, ctx: &NodeContext) {
        let bounds = ctx.bounds_self.clone();
        self.compact.replace_with(Property::computed(
            move || bounds.get().0 < 700.0,
            &[ctx.bounds_self.untyped()],
        ));
        let compact = self.compact.clone();
        let safe_left = self.safe_left.clone();
        let safe_right = self.safe_right.clone();
        self.gutter.replace_with(Property::computed(
            move || {
                let base = if compact.get() { 20.0 } else { 48.0 };
                base + safe_left.get().max(safe_right.get())
            },
            &[
                self.compact.untyped(),
                self.safe_left.untyped(),
                self.safe_right.untyped(),
            ],
        ));
        let compact = self.compact.clone();
        self.card_width.replace_with(Property::computed(
            move || if compact.get() { 236.0 } else { 292.0 },
            &[self.compact.untyped()],
        ));
        let bounds = ctx.bounds_self.clone();
        self.hero_height.replace_with(Property::computed(
            move || {
                let (width, height) = bounds.get();
                if width < 700.0 {
                    354.0
                } else {
                    (height * 0.52).clamp(360.0, 520.0)
                }
            },
            &[ctx.bounds_self.untyped()],
        ));
        let width = self.card_width.clone();
        self.row_height.replace_with(Property::computed(
            move || width.get() * 0.75 + 126.0,
            &[self.card_width.untyped()],
        ));
        let movies = catalog();
        let rows = shelves(&movies);
        self.shelf_count.set(rows.len());
        self.shelves.set(rows);
        let id = self.selected_id.clone();
        self.selected.replace_with(Property::computed(
            move || movies[id.get()].clone(),
            &[self.selected_id.untyped()],
        ));
    }

    pub fn open_feature(&mut self, _ctx: &NodeContext, _event: Event<Click>) {
        if !self.modal_open.get() {
            self.selected_id.set(0);
            self.modal_open.set(true);
        }
    }

    pub fn close(&mut self, _ctx: &NodeContext, _event: Event<Click>) {
        self.modal_open.set(false);
    }

    pub fn toggle_profile(&mut self, _ctx: &NodeContext, _event: Event<Click>) {
        self.profile_open.set(!self.profile_open.get());
    }

    pub fn close_profile(&mut self, _ctx: &NodeContext, _event: Event<Click>) {
        self.profile_open.set(false);
    }

    pub fn key_down(&mut self, _ctx: &NodeContext, event: Event<KeyDown>) {
        if self.modal_open.get() || self.profile_open.get() {
            if event.keyboard.key == "Escape" {
                self.modal_open.set(false);
                self.profile_open.set(false);
            }
            event.prevent_default();
        }
    }
}

#[pax]
#[file("theme.pax")]
pub struct CinemaTheme {
    pub is_dark: Property<bool>,
}
