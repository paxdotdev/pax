#![allow(unused_imports)]

use pax_kit::*;

use crate::{build_dot_texture, dot_texture_dimensions};

#[pax]
#[file("dot_card.pax")]
pub struct DotCard {
    pub index: Property<usize>,
    pub texture: Property<Vec<u8>>,
}

impl DotCard {
    pub fn on_mount(&mut self, _ctx: &NodeContext) {
        let (width, height) = dot_texture_dimensions();
        let card_index = self.index.get();
        let clamped_index = if card_index > 0 { card_index - 1 } else { 0 };
        self.texture
            .set(build_dot_texture(clamped_index, width, height));
    }
}
