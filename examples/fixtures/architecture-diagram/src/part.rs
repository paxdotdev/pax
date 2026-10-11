use crate::PartData;
use pax_kit::pax_engine::api::cursor::CursorStyle;
use pax_kit::*;

#[pax]
#[file("part.pax")]
pub struct DiagramPart {
    pub part: Property<PartData>,
    pub selected: Property<String>,
}

impl DiagramPart {
    pub fn hover(&mut self, ctx: &NodeContext, _: Event<MouseOver>) {
        ctx.set_cursor(CursorStyle::Pointer);
    }
    pub fn leave(&mut self, ctx: &NodeContext, _: Event<MouseOut>) {
        ctx.set_cursor(CursorStyle::Auto);
    }
    pub fn select(&mut self, _: &NodeContext, _: Event<Click>) {
        self.selected.set(self.part.get().id);
    }
}
