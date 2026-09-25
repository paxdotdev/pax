use pax_kit::*;

#[pax]
#[file("compositing_card.pax")]
pub struct CompositingCard {
    pub show_image: Property<bool>,
    pub pixels: Property<Vec<u8>>,
    pub note: Property<String>,
    pub alternate_theme: Property<bool>,
    pub mounts: Property<u64>,
    pub unmounts: Property<u64>,
}

impl CompositingCard {
    pub fn mount(&mut self, _ctx: &NodeContext) {
        self.mounts.set(self.mounts.get() + 1);
    }

    pub fn unmount(&mut self, _ctx: &NodeContext) {
        self.unmounts.set(self.unmounts.get() + 1);
    }
}
