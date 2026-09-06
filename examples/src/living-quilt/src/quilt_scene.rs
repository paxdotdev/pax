use crate::quilt_tile::TileState;
use pax_kit::*;

#[pax]
#[file("quilt_scene.pax")]
pub struct QuiltScene {
    pub is_compact: Property<bool>,
    pub tiles: Property<Vec<TileState>>,
    pub colorized: Property<bool>,
}
