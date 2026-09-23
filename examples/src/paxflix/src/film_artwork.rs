#![allow(unused_imports)]

use crate::CinemaTheme;
use pax_kit::*;

#[pax]
#[file("film_artwork.pax")]
pub struct FilmArtwork {
    pub light_mode: Property<bool>,
    pub still: Property<String>,
    pub thumbnail: Property<String>,
    pub fade_right: Property<bool>,
}
